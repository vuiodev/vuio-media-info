use crate::core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

pub const AC3_SAMPLE_RATES: [u32; 4] = [48000, 44100, 32000, 0];
pub const AC3_BITRATES_KBPS: [u32; 38] = [
    32, 32, 40, 40, 48, 48, 56, 56, 64, 64, 80, 80, 96, 96, 112, 112, 128, 128, 160, 160, 192, 192,
    224, 224, 256, 256, 320, 320, 384, 384, 448, 448, 512, 512, 576, 576, 640, 640,
];

/// Parsed Dolby Digital (AC-3) or Dolby Digital Plus (E-AC-3) frame header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ac3Header {
    pub is_eac3: bool,
    pub sample_rate: u32,
    pub bit_rate: u64,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub dialnorm_db: i32,
    pub has_lfe: bool,
    pub dolby_atmos_present: bool,
    pub frame_size: usize,
}

impl Ac3Header {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        let syncword = u16::from_be_bytes([data[0], data[1]]);
        if syncword != 0x0B77 {
            return Err(MediaInfoError::InvalidSyncword {
                expected: 0x0B77,
                actual: syncword as u32,
            });
        }

        let mut r = MsbBitReader::new(&data[2..]);

        let is_eac3;
        let sample_rate;
        let mut bit_rate = 384000;
        let channels;
        let channel_layout;
        let dialnorm_db;
        let has_lfe;
        let mut dolby_atmos_present = false;
        let mut frame_size = 1536;

        let _crc1 = r.read_bits(16)?;
        let fscod = r.read_bits(2)? as usize;
        let frmsizecod = r.read_bits(6)? as usize;
        let bsid = r.read_bits(5)? as u8;

        if bsid <= 8 {
            is_eac3 = false;
            sample_rate = AC3_SAMPLE_RATES[fscod];
            if frmsizecod < AC3_BITRATES_KBPS.len() {
                bit_rate = AC3_BITRATES_KBPS[frmsizecod] as u64 * 1000;
            }

            let _bsmod = r.read_bits(3)?;
            let acmod = r.read_bits(3)? as u8;

            if (acmod & 0x01) != 0 && acmod != 1 {
                let _cmixlev = r.read_bits(2)?;
            }
            if (acmod & 0x04) != 0 {
                let _surmixlev = r.read_bits(2)?;
            }
            if acmod == 2 {
                let _dsurmod = r.read_bits(2)?;
            }

            has_lfe = r.read_bit()?;
            let dialnorm = r.read_bits(5)? as i32;
            dialnorm_db = -31 + dialnorm;

            let (base_channels, layout) = Self::acmod_to_layout(acmod, has_lfe);
            channels = base_channels;
            channel_layout = layout;
        } else {
            is_eac3 = true;
            let mut r2 = MsbBitReader::new(&data[2..]);
            let _strmtyp = r2.read_bits(2)?;
            let _substreamid = r2.read_bits(3)?;
            let frmsiz = r2.read_bits(11)? as usize;
            frame_size = (frmsiz + 1) * 2;

            let fscod_eac3 = r2.read_bits(2)? as usize;
            if fscod_eac3 == 3 {
                let _fscod2 = r2.read_bits(2)?;
                sample_rate = 24000;
            } else {
                let _numblkscod = r2.read_bits(2)?;
                sample_rate = AC3_SAMPLE_RATES[fscod_eac3];
            }

            let acmod = r2.read_bits(3)? as u8;
            has_lfe = r2.read_bit()?;
            let _bsid_eac3 = r2.read_bits(5)?;
            let dialnorm = r2.read_bits(5)? as i32;
            dialnorm_db = -31 + dialnorm;

            let (base_channels, layout) = Self::acmod_to_layout(acmod, has_lfe);
            channels = base_channels;
            channel_layout = layout;

            let search_window = &data[..data.len().min(4096)];
            if search_window
                .windows(2)
                .any(|w| w == [0x77, 0x0B] || w == [0xA5, 0x5A])
            {
                dolby_atmos_present = true;
            }
        }

        Ok(Self {
            is_eac3,
            sample_rate,
            bit_rate,
            channels,
            channel_layout,
            dialnorm_db,
            has_lfe,
            dolby_atmos_present,
            frame_size,
        })
    }

    fn acmod_to_layout(acmod: u8, has_lfe: bool) -> (u32, AudioChannelLayout) {
        let (chans, layout) = match acmod {
            0 => (2, AudioChannelLayout::Stereo),
            1 => (1, AudioChannelLayout::Mono),
            2 => (2, AudioChannelLayout::Stereo),
            3 => (3, AudioChannelLayout::Surround3_0),
            4 => (3, AudioChannelLayout::Surround3_0),
            5 => (4, AudioChannelLayout::Surround4_0),
            6 => (4, AudioChannelLayout::Surround4_0),
            7 => (5, AudioChannelLayout::Surround5_1),
            _ => (2, AudioChannelLayout::Stereo),
        };

        if has_lfe {
            (chans + 1, layout)
        } else {
            (chans, layout)
        }
    }
}
