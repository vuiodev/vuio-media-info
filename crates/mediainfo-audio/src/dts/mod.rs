use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

pub const DTS_SAMPLE_RATES: [u32; 16] = [
    0, 8000, 16000, 32000, 0, 0, 11025, 22050, 44100, 0, 0, 12000, 24000, 48000, 96000, 192000,
];

pub const DTS_BITRATES_KBPS: [u32; 32] = [
    32, 56, 64, 96, 112, 128, 192, 224, 256, 320, 384, 448, 512, 576, 640, 768, 896, 1024, 1152,
    1280, 1344, 1408, 1411, 1472, 1536, 1920, 2048, 3072, 3840, 0, 0, 0,
];

/// Parsed DTS Audio Header.
#[derive(Debug, Clone, PartialEq)]
pub struct DtsHeader {
    pub profile_name: &'static str,
    pub sample_rate: u32,
    pub bit_rate: u64,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub is_dtshd_ma: bool,
    pub is_dtsx: bool,
    pub bit_depth: u8,
}

impl DtsHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 16 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 16,
                actual: data.len(),
            });
        }

        let sync = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        if sync != 0x7FFE8001 && sync != 0xFE7F0180 && sync != 0x1FFFE800 && sync != 0xFF1F00E8 {
            return Err(MediaInfoError::InvalidSyncword {
                expected: 0x7FFE8001,
                actual: sync,
            });
        }

        let mut r = MsbBitReader::new(&data[4..]);

        let _frame_type = r.read_bit()?;
        let _deficit_samples = r.read_bits(5)?;
        let _crc_present = r.read_bit()?;
        let _pcm_blocks = r.read_bits(7)?;
        let _frame_bytes = r.read_bits(14)?;
        let audio_channel_arrangement = r.read_bits(6)? as u8;
        let sfreq = r.read_bits(4)? as usize;
        let rate = r.read_bits(5)? as usize;

        let sample_rate = if sfreq < DTS_SAMPLE_RATES.len() {
            DTS_SAMPLE_RATES[sfreq]
        } else {
            48000
        };

        let bit_rate = if rate < DTS_BITRATES_KBPS.len() && DTS_BITRATES_KBPS[rate] > 0 {
            DTS_BITRATES_KBPS[rate] as u64 * 1000
        } else {
            1509000
        };

        let (channels, channel_layout) = match audio_channel_arrangement {
            0 => (1, AudioChannelLayout::Mono),
            1 => (2, AudioChannelLayout::Stereo), // Dual Mono
            2 => (2, AudioChannelLayout::Stereo),
            9 => (6, AudioChannelLayout::Surround5_1), // 3/2 + LFE
            _ => (6, AudioChannelLayout::Surround5_1),
        };

        // Check for DTS-HD Master Audio extension (0x64582025) or DTS:X (0x41A29547)
        let is_dtshd_ma = data.windows(4).any(|w| w == [0x64, 0x58, 0x20, 0x25]);
        let is_dtsx = data.windows(4).any(|w| w == [0x41, 0xA2, 0x95, 0x47]);

        let profile_name = if is_dtsx {
            "DTS:X / DTS-HD MA"
        } else if is_dtshd_ma {
            "DTS-HD Master Audio"
        } else {
            "DTS Digital Surround"
        };

        Ok(Self {
            profile_name,
            sample_rate,
            bit_rate,
            channels,
            channel_layout,
            is_dtshd_ma,
            is_dtsx,
            bit_depth: if is_dtshd_ma { 24 } else { 16 },
        })
    }
}
