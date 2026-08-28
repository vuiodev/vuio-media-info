use crate::core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

pub const MP3_BITRATES_V1_L1: [u32; 16] = [
    0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
];
pub const MP3_BITRATES_V1_L2: [u32; 16] = [
    0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
];
pub const MP3_BITRATES_V1_L3: [u32; 16] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
];
pub const MP3_BITRATES_V2_L1: [u32; 16] = [
    0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
];
pub const MP3_BITRATES_V2_L23: [u32; 16] = [
    0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
];

pub const MP3_SAMPLE_RATES_V1: [u32; 4] = [44100, 48000, 32000, 0];
pub const MP3_SAMPLE_RATES_V2: [u32; 4] = [22050, 24000, 16000, 0];
pub const MP3_SAMPLE_RATES_V2_5: [u32; 4] = [11025, 12000, 8000, 0];

/// Parsed MPEG Audio frame header.
#[derive(Debug, Clone, PartialEq)]
pub struct MpegaHeader {
    pub version: &'static str,
    pub layer: &'static str,
    pub sample_rate: u32,
    pub bit_rate: u64,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub is_vbr: bool,
    pub frame_size: usize,
    pub xing_frames: Option<u32>,
    pub xing_bytes: Option<u32>,
}

impl MpegaHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 4,
                actual: data.len(),
            });
        }

        let mut r = MsbBitReader::new(data);
        let syncword = r.read_bits(11)?;
        if syncword != 0x7FF {
            return Err(MediaInfoError::InvalidSyncword {
                expected: 0x7FF,
                actual: syncword,
            });
        }

        let version_id = r.read_bits(2)?; // 0: MPEG 2.5, 2: MPEG 2, 3: MPEG 1
        let layer_id = r.read_bits(2)?; // 1: Layer III, 2: Layer II, 3: Layer I
        let _protection_bit = r.read_bit()?;
        let bitrate_idx = (r.read_bits(4)? as usize).min(15);
        let sampling_idx = (r.read_bits(2)? as usize).min(3);
        let padding_bit = r.read_bit()?;
        let _private_bit = r.read_bit()?;
        let channel_mode = r.read_bits(2)?;

        let version = match version_id {
            0 => "Version 2.5",
            2 => "Version 2",
            _ => "Version 1",
        };

        let layer = match layer_id {
            1 => "Layer 3",
            2 => "Layer 2",
            3 => "Layer 1",
            _ => "Layer 3",
        };

        let sample_rate = match version_id {
            0 => MP3_SAMPLE_RATES_V2_5[sampling_idx],
            2 => MP3_SAMPLE_RATES_V2[sampling_idx],
            _ => MP3_SAMPLE_RATES_V1[sampling_idx],
        };

        let bit_rate_kbps = match (version_id == 3, layer_id) {
            (true, 3) => MP3_BITRATES_V1_L1[bitrate_idx],
            (true, 2) => MP3_BITRATES_V1_L2[bitrate_idx],
            (true, 1) => MP3_BITRATES_V1_L3[bitrate_idx],
            (true, _) => MP3_BITRATES_V1_L3[bitrate_idx],
            (false, 3) => MP3_BITRATES_V2_L1[bitrate_idx],
            (false, _) => MP3_BITRATES_V2_L23[bitrate_idx],
        };
        let bit_rate = (if bit_rate_kbps > 0 {
            bit_rate_kbps
        } else {
            128
        }) as u64
            * 1000;

        let (channels, channel_layout) = match channel_mode {
            3 => (1, AudioChannelLayout::Mono),
            _ => (2, AudioChannelLayout::Stereo),
        };

        let samples_per_frame: u32 = match (version_id == 3, layer_id) {
            (_, 3) => 384,
            (true, _) => 1152,
            (false, _) => 576,
        };

        let frame_size = if sample_rate > 0 && bit_rate > 0 {
            let padding = if padding_bit { 1 } else { 0 };
            if layer_id == 3 {
                // Layer 1
                (((12 * bit_rate / sample_rate as u64) + padding as u64) * 4) as usize
            } else {
                // Layer 2 or 3
                ((samples_per_frame as u64 * bit_rate / (8 * sample_rate as u64)) + padding as u64)
                    as usize
            }
        } else {
            418
        };

        // Check for Xing / Info / VBRI VBR header inside the first frame
        let mut is_vbr = false;
        let mut xing_frames = None;
        let mut xing_bytes = None;

        let scan_len = data.len().min(frame_size.max(64) + 64);
        let frame_slice = &data[..scan_len];

        for i in 0..frame_slice.len().saturating_sub(16) {
            if &frame_slice[i..i + 4] == b"Xing" || &frame_slice[i..i + 4] == b"Info" {
                is_vbr = &frame_slice[i..i + 4] == b"Xing";
                if i + 12 <= frame_slice.len() {
                    let flags = u32::from_be_bytes([
                        frame_slice[i + 4],
                        frame_slice[i + 5],
                        frame_slice[i + 6],
                        frame_slice[i + 7],
                    ]);
                    let mut pos = i + 8;
                    if flags & 0x01 != 0 && pos + 4 <= frame_slice.len() {
                        xing_frames = Some(u32::from_be_bytes([
                            frame_slice[pos],
                            frame_slice[pos + 1],
                            frame_slice[pos + 2],
                            frame_slice[pos + 3],
                        ]));
                        pos += 4;
                    }
                    if flags & 0x02 != 0 && pos + 4 <= frame_slice.len() {
                        xing_bytes = Some(u32::from_be_bytes([
                            frame_slice[pos],
                            frame_slice[pos + 1],
                            frame_slice[pos + 2],
                            frame_slice[pos + 3],
                        ]));
                    }
                }
                break;
            } else if &frame_slice[i..i + 4] == b"VBRI" {
                is_vbr = true;
                if i + 18 <= frame_slice.len() {
                    xing_bytes = Some(u32::from_be_bytes([
                        frame_slice[i + 10],
                        frame_slice[i + 11],
                        frame_slice[i + 12],
                        frame_slice[i + 13],
                    ]));
                    xing_frames = Some(u32::from_be_bytes([
                        frame_slice[i + 14],
                        frame_slice[i + 15],
                        frame_slice[i + 16],
                        frame_slice[i + 17],
                    ]));
                }
                break;
            }
        }

        Ok(Self {
            version,
            layer,
            sample_rate,
            bit_rate,
            channels,
            channel_layout,
            is_vbr,
            frame_size,
            xing_frames,
            xing_bytes,
        })
    }
}
