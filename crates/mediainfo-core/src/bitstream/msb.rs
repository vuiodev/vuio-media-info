use crate::error::{MediaInfoError, Result};
use memchr::memmem;

/// Zero-copy MSB-first bit reader operating on byte slices.
/// Ideal for ISO/IEC bitstreams (H.264, H.265, AV1, MP4 boxes, MPEG, AC-3, AAC ADTS).
#[derive(Debug, Clone)]
pub struct MsbBitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8, // 0 to 7 (0 = MSB bit 7, 7 = LSB bit 0)
}

impl<'a> MsbBitReader<'a> {
    /// Create a new MsbBitReader wrapping the given byte slice.
    #[inline(always)]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Total bits remaining in the buffer.
    #[inline(always)]
    pub fn remaining_bits(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            0
        } else {
            (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
        }
    }

    /// Total whole bytes remaining in the buffer from current byte alignment.
    #[inline(always)]
    pub fn remaining_bytes(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            0
        } else {
            self.data.len() - self.byte_offset - if self.bit_offset > 0 { 1 } else { 0 }
        }
    }

    /// Number of bytes read so far (rounded up if bit offset is non-zero).
    #[inline(always)]
    pub fn bytes_read(&self) -> usize {
        self.byte_offset + if self.bit_offset > 0 { 1 } else { 0 }
    }

    /// Current bit position in the entire slice.
    #[inline(always)]
    pub fn bit_position(&self) -> usize {
        self.byte_offset * 8 + self.bit_offset as usize
    }

    /// Read a single bit (0 or 1) as a boolean.
    #[inline(always)]
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let bit = (self.data[self.byte_offset] >> (7 - self.bit_offset)) & 1;
        self.bit_offset += 1;
        if self.bit_offset == 8 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }

        Ok(bit == 1)
    }

    /// Read up to 32 bits into a u32.
    #[inline(always)]
    pub fn read_bits(&mut self, count: u8) -> Result<u32> {
        if count == 0 {
            return Ok(0);
        }
        if count > 32 {
            return Err(MediaInfoError::BitReaderError(
                "read_bits supports up to 32 bits".to_string(),
            ));
        }

        if self.remaining_bits() < count as usize {
            return Err(MediaInfoError::UnexpectedEof {
                expected: (count as usize).div_ceil(8),
                actual: self.remaining_bytes(),
            });
        }

        let mut result = 0u32;
        let mut bits_needed = count;

        while bits_needed > 0 {
            let bits_in_current_byte = 8 - self.bit_offset;
            let bits_to_take = bits_needed.min(bits_in_current_byte);

            let mask = if bits_to_take == 8 {
                0xFFu32
            } else {
                (1u32 << bits_to_take) - 1
            };
            let shift = bits_in_current_byte - bits_to_take;
            let chunk = (self.data[self.byte_offset] as u32 >> shift) & mask;

            result = (result << bits_to_take) | chunk;

            self.bit_offset += bits_to_take;
            if self.bit_offset == 8 {
                self.bit_offset = 0;
                self.byte_offset += 1;
            }

            bits_needed -= bits_to_take;
        }

        Ok(result)
    }

    /// Read up to 64 bits into a u64.
    #[inline(always)]
    pub fn read_bits_u64(&mut self, count: u8) -> Result<u64> {
        if count == 0 {
            return Ok(0);
        }
        if count > 64 {
            return Err(MediaInfoError::BitReaderError(
                "read_bits_u64 supports up to 64 bits".to_string(),
            ));
        }

        if count <= 32 {
            self.read_bits(count).map(|v| v as u64)
        } else {
            let high = self.read_bits(32)? as u64;
            let low = self.read_bits(count - 32)? as u64;
            Ok((high << (count - 32)) | low)
        }
    }

    /// Peek up to 32 bits without advancing the reader position.
    #[inline(always)]
    pub fn peek_bits(&self, count: u8) -> Result<u32> {
        let mut clone = self.clone();
        clone.read_bits(count)
    }

    /// Skip `count` bits.
    #[inline(always)]
    pub fn skip_bits(&mut self, count: usize) -> Result<()> {
        if self.remaining_bits() < count {
            return Err(MediaInfoError::UnexpectedEof {
                expected: count.div_ceil(8),
                actual: self.remaining_bytes(),
            });
        }

        let total_bits = self.bit_offset as usize + count;
        self.byte_offset += total_bits / 8;
        self.bit_offset = (total_bits % 8) as u8;
        Ok(())
    }

    /// Align to the next byte boundary by skipping any sub-byte bits.
    #[inline(always)]
    pub fn byte_align(&mut self) {
        if self.bit_offset > 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Read an Unsigned Exponential-Golomb (ue) coded integer using hardware CLZ (Count Leading Zeros).
    /// Emits single-cycle CLZ (ARM64 NEON) or LZCNT/BSR (x86_64 AVX2).
    #[inline(always)]
    pub fn read_ue(&mut self) -> Result<u32> {
        let peek_bits = (self.remaining_bits().min(32)) as u8;
        if peek_bits == 0 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let peeked = self.peek_bits(peek_bits)?;
        let leading_zeros = if peeked == 0 {
            // More than 32 leading zeros: loop fallback
            let mut zeros = peek_bits;
            self.skip_bits(peek_bits as usize)?;
            while !self.read_bit()? {
                zeros += 1;
                if zeros > 31 {
                    return Err(MediaInfoError::BitReaderError(
                        "Exp-Golomb code overflow (> 31 leading zeros)".to_string(),
                    ));
                }
            }
            zeros
        } else {
            // Single CPU hardware CLZ instruction
            let shifted = peeked << (32 - peek_bits);
            let lz = shifted.leading_zeros() as u8;
            self.skip_bits(lz as usize + 1)?;
            lz
        };

        if leading_zeros == 0 {
            return Ok(0);
        }

        let info = self.read_bits(leading_zeros)?;
        let value = (1u32 << leading_zeros) - 1 + info;
        Ok(value)
    }

    /// Read a Signed Exponential-Golomb (se) coded integer.
    #[inline(always)]
    pub fn read_se(&mut self) -> Result<i32> {
        let code_num = self.read_ue()?;
        if code_num % 2 == 1 {
            Ok(code_num.div_ceil(2) as i32)
        } else {
            Ok(-((code_num / 2) as i32))
        }
    }

    /// Read a raw slice of `count` bytes. Byte aligns the reader before reading.
    #[inline(always)]
    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        self.byte_align();
        if self.byte_offset + count > self.data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: count,
                actual: self.data.len().saturating_sub(self.byte_offset),
            });
        }

        let slice = &self.data[self.byte_offset..self.byte_offset + count];
        self.byte_offset += count;
        Ok(slice)
    }

    /// Read standard unsigned integers (Big-Endian).
    #[inline(always)]
    pub fn read_u8(&mut self) -> Result<u8> {
        self.read_bits(8).map(|v| v as u8)
    }

    #[inline(always)]
    pub fn read_u16_be(&mut self) -> Result<u16> {
        self.read_bits(16).map(|v| v as u16)
    }

    #[inline(always)]
    pub fn read_u24_be(&mut self) -> Result<u32> {
        self.read_bits(24)
    }

    #[inline(always)]
    pub fn read_u32_be(&mut self) -> Result<u32> {
        self.read_bits(32)
    }

    #[inline(always)]
    pub fn read_u64_be(&mut self) -> Result<u64> {
        self.read_bits_u64(64)
    }
}

/// Strip H.264 / H.265 emulation prevention three bytes (`0x000003` -> `0x0000`).
/// Uses hardware SIMD (AVX2 / ARM NEON) via vector substring scanning.
pub fn unescape_nal_unit(data: &[u8]) -> Vec<u8> {
    // Fast path: if no 0x000003 sequence exists, copy directly in bulk without byte loop
    let finder = memmem::Finder::new(b"\x00\x00\x03");
    if finder.find(data).is_none() {
        return data.to_vec();
    }

    let mut unescaped = Vec::with_capacity(data.len());
    let mut cursor = 0;

    for match_idx in finder.find_iter(data) {
        if match_idx >= cursor {
            // Append data up to 0x00 0x00
            unescaped.extend_from_slice(&data[cursor..match_idx + 2]);
            // Skip the 0x03 emulation prevention byte
            cursor = match_idx + 3;
        }
    }

    if cursor < data.len() {
        unescaped.extend_from_slice(&data[cursor..]);
    }

    unescaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msb_read_bits() {
        let data = [0b10101100, 0b11110000];
        let mut reader = MsbBitReader::new(&data);

        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert_eq!(reader.read_bits(4).unwrap(), 0b1011);
        assert_eq!(reader.read_bits(2).unwrap(), 0b00);
        assert_eq!(reader.read_bits(8).unwrap(), 0b11110000);
        assert_eq!(reader.remaining_bits(), 0);
    }

    #[test]
    fn test_exp_golomb_clz() {
        let data1 = [0b10000000];
        let mut reader1 = MsbBitReader::new(&data1);
        assert_eq!(reader1.read_ue().unwrap(), 0);

        let data2 = [0b01000000];
        let mut reader2 = MsbBitReader::new(&data2);
        assert_eq!(reader2.read_ue().unwrap(), 1);

        let data3 = [0b01100000];
        let mut reader3 = MsbBitReader::new(&data3);
        assert_eq!(reader3.read_ue().unwrap(), 2);

        let data4 = [0b00111000];
        let mut reader4 = MsbBitReader::new(&data4);
        assert_eq!(reader4.read_ue().unwrap(), 6);
    }

    #[test]
    fn test_unescape_nal_simd() {
        let raw = [0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x02];
        let unescaped = unescape_nal_unit(&raw);
        assert_eq!(unescaped, vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x02]);

        let plain = [0x01, 0x02, 0x03, 0x04, 0x05];
        assert_eq!(unescape_nal_unit(&plain), plain.to_vec());
    }
}
