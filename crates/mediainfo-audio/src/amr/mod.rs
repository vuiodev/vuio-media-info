use mediainfo_core::error::{MediaInfoError, Result};

/// Parsed Adaptive Multi-Rate (AMR-NB / AMR-WB) stream.
#[derive(Debug, Clone, PartialEq)]
pub struct AmrInfo {
    pub is_wideband: bool,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_depth: u8,
    pub duration_ms: Option<f64>,
    pub bit_rate: Option<u64>,
    pub format_profile: String,
}

pub const AMR_NB_MAGIC: [u8; 6] = *b"#!AMR\n";
pub const AMR_WB_MAGIC: [u8; 9] = *b"#!AMR-WB\n";

const AMR_NB_FRAME_SIZES: [usize; 9] = [13, 14, 16, 18, 20, 21, 27, 32, 6];
const AMR_WB_FRAME_SIZES: [usize; 10] = [18, 24, 33, 37, 41, 47, 51, 59, 61, 6];

impl AmrInfo {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let (is_wideband, mut offset) = if data.starts_with(&AMR_WB_MAGIC) {
            (true, 9)
        } else if data.starts_with(&AMR_NB_MAGIC) {
            (false, 6)
        } else {
            return Err(MediaInfoError::InvalidData("Not a valid AMR file".to_string()));
        };

        let sample_rate = if is_wideband { 16000 } else { 8000 };
        let format_profile = if is_wideband {
            "Adaptive Multi-Rate Wideband (AMR-WB)".to_string()
        } else {
            "Adaptive Multi-Rate Narrowband (AMR-NB)".to_string()
        };

        // Scan frames to count duration (each speech frame is exactly 20 ms)
        let mut frame_count = 0u64;
        while offset < data.len() {
            let header_byte = data[offset];
            let mode = if is_wideband {
                ((header_byte >> 3) & 0x0F) as usize
            } else {
                ((header_byte >> 3) & 0x0F) as usize
            };

            let frame_size = if is_wideband {
                if mode < AMR_WB_FRAME_SIZES.len() { AMR_WB_FRAME_SIZES[mode] } else { 1 }
            } else {
                if mode < AMR_NB_FRAME_SIZES.len() { AMR_NB_FRAME_SIZES[mode] } else { 1 }
            };

            offset += frame_size;
            frame_count += 1;
        }

        let duration_ms = if frame_count > 0 {
            Some(frame_count as f64 * 20.0)
        } else {
            None
        };

        let bit_rate = if let Some(dur) = duration_ms {
            if dur > 0.0 {
                Some(((data.len() as u64 * 8) as f64 / (dur / 1000.0)) as u64)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            is_wideband,
            sample_rate,
            channels: 1,
            bit_depth: 16,
            duration_ms,
            bit_rate,
            format_profile,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amr_stream() {
        let mut data = Vec::new();
        data.extend_from_slice(&AMR_NB_MAGIC);
        // Add 5 frames of Mode 7 (32 bytes each) -> 5 * 20ms = 100ms
        for _ in 0..5 {
            let mut frame = vec![0u8; 32];
            frame[0] = 7 << 3;
            data.extend_from_slice(&frame);
        }

        let amr = AmrInfo::parse(&data).unwrap();
        assert!(!amr.is_wideband);
        assert_eq!(amr.sample_rate, 8000);
        assert_eq!(amr.duration_ms, Some(100.0));
    }
}
