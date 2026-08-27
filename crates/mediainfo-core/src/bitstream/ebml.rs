use crate::error::{MediaInfoError, Result};

/// EBML Variable Length Integer (VINT) parser for Matroska / WebM with hardware-accelerated CLZ.
pub struct EbmlVint;

impl EbmlVint {
    /// Read an EBML Element ID from `data` at `offset`.
    /// Returns `(element_id: u32, bytes_consumed: usize)`.
    /// Preserves the leading marker bit in the element ID.
    #[inline(always)]
    pub fn read_element_id(data: &[u8], offset: usize) -> Result<(u32, usize)> {
        if offset >= data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let first_byte = data[offset];
        if first_byte == 0 {
            return Err(MediaInfoError::CorruptBitstream(
                "Invalid EBML Element ID with 0 leading byte".to_string(),
            ));
        }

        let length = first_byte.leading_zeros() as usize + 1;
        if length > 4 {
            return Err(MediaInfoError::CorruptBitstream(format!(
                "EBML Element ID length > 4 ({length})"
            )));
        }

        if offset + length > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: length,
                actual: data.len() - offset,
            });
        }

        let id = match length {
            1 => first_byte as u32,
            2 => u16::from_be_bytes([data[offset], data[offset + 1]]) as u32,
            3 => {
                ((data[offset] as u32) << 16)
                    | ((data[offset + 1] as u32) << 8)
                    | (data[offset + 2] as u32)
            }
            4 => u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]),
            _ => 0,
        };

        Ok((id, length))
    }

    /// Read an EBML Element Size (VINT data size) from `data` at `offset`.
    /// Returns `(size: Option<u64>, bytes_consumed: usize)`.
    /// `None` indicates an unknown/undefined size (all bits 1).
    #[inline(always)]
    pub fn read_element_size(data: &[u8], offset: usize) -> Result<(Option<u64>, usize)> {
        if offset >= data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let first_byte = data[offset];
        if first_byte == 0 {
            return Err(MediaInfoError::CorruptBitstream(
                "Invalid EBML VINT size with 0 leading byte".to_string(),
            ));
        }

        let length = first_byte.leading_zeros() as usize + 1;
        if length > 8 {
            return Err(MediaInfoError::CorruptBitstream(format!(
                "EBML Element Size length > 8 ({length})"
            )));
        }

        if offset + length > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: length,
                actual: data.len() - offset,
            });
        }

        // Mask out the marker bit in the first byte
        let mask = if length >= 8 { 0u8 } else { 0xFFu8 >> length };
        let mut raw_val = (first_byte & mask) as u64;

        for i in 1..length {
            raw_val = (raw_val << 8) | (data[offset + i] as u64);
        }

        // Check for unknown/undefined size (all data bits are 1)
        let unknown_mask = (1u64 << (7 * length)) - 1;
        if raw_val == unknown_mask {
            Ok((None, length))
        } else {
            Ok((Some(raw_val), length))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebml_element_id() {
        let data = [0x1A, 0x45, 0xDF, 0xA3];
        let (id, len) = EbmlVint::read_element_id(&data, 0).unwrap();
        assert_eq!(id, 0x1A45DFA3);
        assert_eq!(len, 4);

        let data2 = [0xAE];
        let (id2, len2) = EbmlVint::read_element_id(&data2, 0).unwrap();
        assert_eq!(id2, 0xAE);
        assert_eq!(len2, 1);
    }

    #[test]
    fn test_ebml_element_size() {
        let data = [0x85]; // 1 byte, value 5 (marker bit 0x80)
        let (size, len) = EbmlVint::read_element_size(&data, 0).unwrap();
        assert_eq!(size, Some(5));
        assert_eq!(len, 1);

        let data_unknown = [0xFF]; // all bits 1 -> unknown size
        let (size_unk, _) = EbmlVint::read_element_size(&data_unknown, 0).unwrap();
        assert_eq!(size_unk, None);
    }
}
