use crate::error::{MediaInfoError, Result};

/// Zero-copy LSB-first bit reader operating on byte slices.
/// Ideal for Vorbis, FLAC entropy decoding, and Little-Endian bitstreams.
#[derive(Debug, Clone)]
pub struct LsbBitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8, // 0 to 7 (0 = LSB bit 0, 7 = MSB bit 7)
}

impl<'a> LsbBitReader<'a> {
    /// Create a new LsbBitReader wrapping the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Total bits remaining in the buffer.
    #[inline]
    pub fn remaining_bits(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            0
        } else {
            (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
        }
    }

    /// Read a single bit (0 or 1) as a boolean.
    #[inline]
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let bit = (self.data[self.byte_offset] >> self.bit_offset) & 1;
        self.bit_offset += 1;
        if self.bit_offset == 8 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }

        Ok(bit == 1)
    }

    /// Read up to 32 bits into a u32 (LSB first).
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
                expected: (count as usize + 7) / 8,
                actual: 0,
            });
        }

        let mut result = 0u32;
        let mut bits_read = 0u8;

        while bits_read < count {
            let bits_in_current_byte = 8 - self.bit_offset;
            let bits_to_take = (count - bits_read).min(bits_in_current_byte);

            let mask = if bits_to_take == 8 {
                0xFFu32
            } else {
                (1u32 << bits_to_take) - 1
            };
            let chunk = (self.data[self.byte_offset] as u32 >> self.bit_offset) & mask;

            result |= chunk << bits_read;

            self.bit_offset += bits_to_take;
            if self.bit_offset == 8 {
                self.bit_offset = 0;
                self.byte_offset += 1;
            }

            bits_read += bits_to_take;
        }

        Ok(result)
    }

    /// Read an unsigned integer (Little-Endian).
    pub fn read_u16_le(&mut self) -> Result<u16> {
        self.read_bits(16).map(|v| v as u16)
    }

    pub fn read_u32_le(&mut self) -> Result<u32> {
        self.read_bits(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsb_read_bits() {
        let data = [0b10101100];
        let mut reader = LsbBitReader::new(&data);

        // First 4 bits: 1100 (LSB first) -> 0b1100 = 12
        assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
        // Next 4 bits: 1010 -> 0b1010 = 10
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
    }
}
