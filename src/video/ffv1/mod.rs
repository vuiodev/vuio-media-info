use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Number of probability states the FFV1 header decoder keeps.
const CONTEXT_SIZE: usize = 32;

/// FFV1 global header, as carried in the codec configuration record.
#[derive(Debug, Clone, PartialEq)]
pub struct Ffv1Header {
    pub version: u32,
    pub micro_version: u32,
    pub coder_type: u32,
    pub colorspace: u32,
    pub bits_per_raw_sample: u8,
    pub chroma_planes: bool,
    pub chroma_h_shift: u32,
    pub chroma_v_shift: u32,
    pub transparency: bool,
}

impl Ffv1Header {
    /// Version string as MediaInfo renders it, for example `3.4`.
    pub fn version_string(&self) -> String {
        if self.version >= 3 {
            format!("{}.{}", self.version, self.micro_version)
        } else {
            self.version.to_string()
        }
    }

    pub fn chroma_subsampling(&self) -> ChromaSubsampling {
        // colorspace 1 is JPEG2000-RCT, which codes RGB planes.
        if self.colorspace == 1 {
            return ChromaSubsampling::RGB;
        }
        if !self.chroma_planes {
            return ChromaSubsampling::Monochrome;
        }
        match (self.chroma_h_shift, self.chroma_v_shift) {
            (0, 0) => ChromaSubsampling::YUV444,
            (1, 0) => ChromaSubsampling::YUV422,
            (1, 1) => ChromaSubsampling::YUV420,
            (2, 0) => ChromaSubsampling::YUV411,
            _ => ChromaSubsampling::YUV420,
        }
    }

    /// Decodes the range-coded global header found in an FFV1 configuration record.
    pub fn parse(extradata: &[u8]) -> Result<Self> {
        if extradata.len() < 4 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 4,
                actual: extradata.len(),
            });
        }

        let mut coder = RangeCoder::new(extradata);
        let mut state = [128u8; CONTEXT_SIZE];

        let version = coder.symbol(&mut state)?;
        if version > 4 {
            return Err(MediaInfoError::InvalidData(format!(
                "Unsupported FFV1 version {version}"
            )));
        }

        // Versions above 2 append a 32-bit CRC that is not part of the coded header.
        let micro_version = if version > 2 {
            coder.trim_end(4);
            coder.symbol(&mut state)?
        } else {
            0
        };

        let coder_type = coder.symbol(&mut state)?;
        if coder_type > 1 {
            // A custom state transition table follows, one delta per state.
            for _ in 1..256 {
                coder.signed_symbol(&mut state)?;
            }
        }

        let colorspace = coder.symbol(&mut state)?;
        let bits_per_raw_sample = if version > 0 {
            coder.symbol(&mut state)?
        } else {
            8
        };
        let chroma_planes = coder.bit(&mut state[0]);
        let chroma_h_shift = coder.symbol(&mut state)?;
        let chroma_v_shift = coder.symbol(&mut state)?;
        let transparency = coder.bit(&mut state[0]);

        Ok(Self {
            version,
            micro_version,
            coder_type,
            colorspace,
            bits_per_raw_sample: if bits_per_raw_sample == 0 {
                8
            } else {
                bits_per_raw_sample.min(16) as u8
            },
            chroma_planes,
            chroma_h_shift,
            chroma_v_shift,
            transparency,
        })
    }
}

/// The binary range decoder FFV1 uses for its header and, optionally, its samples.
struct RangeCoder<'a> {
    data: &'a [u8],
    position: usize,
    end: usize,
    low: u32,
    range: u32,
    zero_state: [u8; 256],
    one_state: [u8; 256],
}

impl<'a> RangeCoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut low = u32::from(data[0]) << 8 | u32::from(data[1]);
        let mut end = data.len();
        if low >= 0xFF00 {
            low = 0xFF00;
            end = 2;
        }
        let mut coder = Self {
            data,
            position: 2,
            end,
            low,
            range: 0xFF00,
            zero_state: [0; 256],
            one_state: [0; 256],
        };
        // FFV1 fixes the adaptation factor at 0.05 and caps probabilities at 248.
        coder.build_states((0.05 * (1u64 << 32) as f64) as i64, 256 - 8);
        coder
    }

    /// Drops trailing bytes that are not part of the coded stream.
    fn trim_end(&mut self, bytes: usize) {
        self.end = self.end.saturating_sub(bytes);
    }

    fn build_states(&mut self, factor: i64, max_p: usize) {
        let one: i64 = 1 << 32;
        let mut p: i64 = one / 2;
        let mut last_p8: usize = 0;

        for _ in 0..128 {
            let mut p8 = ((256 * p + one / 2) >> 32) as usize;
            if p8 <= last_p8 {
                p8 = last_p8 + 1;
            }
            if last_p8 != 0 && last_p8 < 256 && p8 <= max_p {
                self.one_state[last_p8] = p8 as u8;
            }
            p += ((one - p) * factor + one / 2) >> 32;
            last_p8 = p8;
        }

        for i in (256 - max_p)..=max_p {
            if self.one_state[i] != 0 {
                continue;
            }
            let mut p = ((i as i64) * one + 128) >> 8;
            p += ((one - p) * factor + one / 2) >> 32;
            let mut p8 = ((256 * p + one / 2) >> 32) as usize;
            if p8 <= i {
                p8 = i + 1;
            }
            if p8 > max_p {
                p8 = max_p;
            }
            self.one_state[i] = p8 as u8;
        }

        for i in 1..255usize {
            self.zero_state[i] = 256u16.saturating_sub(self.one_state[256 - i] as u16) as u8;
        }
    }

    fn refill(&mut self) {
        if self.range < 0x100 {
            self.range <<= 8;
            self.low <<= 8;
            if self.position < self.end {
                self.low += u32::from(self.data[self.position]);
            }
            self.position += 1;
        }
    }

    fn bit(&mut self, state: &mut u8) -> bool {
        let range1 = (self.range * u32::from(*state)) >> 8;
        self.range -= range1;
        if self.low < self.range {
            *state = self.zero_state[*state as usize];
            self.refill();
            false
        } else {
            self.low -= self.range;
            *state = self.one_state[*state as usize];
            self.range = range1;
            self.refill();
            true
        }
    }

    /// Reads an unsigned Elias-gamma-style symbol using the FFV1 context layout.
    fn symbol(&mut self, state: &mut [u8; CONTEXT_SIZE]) -> Result<u32> {
        Ok(self.read_symbol(state, false)? as u32)
    }

    fn signed_symbol(&mut self, state: &mut [u8; CONTEXT_SIZE]) -> Result<i32> {
        self.read_symbol(state, true)
    }

    fn read_symbol(&mut self, state: &mut [u8; CONTEXT_SIZE], signed: bool) -> Result<i32> {
        if self.bit(&mut state[0]) {
            return Ok(0);
        }

        let mut exponent = 0usize;
        while self.bit(&mut state[1 + exponent.min(9)]) {
            exponent += 1;
            if exponent > 31 {
                return Err(MediaInfoError::InvalidData(
                    "FFV1 symbol exponent out of range".to_string(),
                ));
            }
        }

        let mut value: u32 = 1;
        for i in (0..exponent).rev() {
            value = value * 2 + u32::from(self.bit(&mut state[22 + i.min(9)]));
        }

        if signed && self.bit(&mut state[11 + exponent.min(10)]) {
            Ok(-(value as i32))
        } else {
            Ok(value as i32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffv1_range_state_tables() {
        // The zero and one transition tables must be mirror images of each other.
        let data = [0u8, 0u8, 0u8, 0u8];
        let coder = RangeCoder::new(&data);
        for i in 1..255usize {
            assert_eq!(
                coder.zero_state[i],
                (256 - coder.one_state[256 - i] as u16) as u8
            );
        }
    }
}
