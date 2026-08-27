use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Audio stream metadata track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioTrack {
    pub stream_id: u32,
    pub stream_order: Option<u32>,
    pub format: AudioCodec,
    pub format_info: Option<String>,
    pub format_profile: Option<String>,
    pub format_commercial: Option<String>,
    pub format_additional_features: Option<String>,
    pub codec_id: Option<String>,
    pub codec_id_info: Option<String>,
    pub duration_ms: Option<f64>,
    pub bit_rate: Option<u64>,
    pub bit_rate_mode: Option<BitrateMode>,
    pub bit_rate_maximum: Option<u64>,
    pub channels: u32,
    pub channel_layout: Option<AudioChannelLayout>,
    pub channel_positions: Option<String>, // e.g. "Front: L C R, Side: L R, LFE"
    pub samples_per_frame: Option<u32>,
    pub sampling_rate: u32, // e.g. 44100, 48000, 96000, 192000
    pub sampling_count: Option<u64>,
    pub frame_rate: Option<f64>,
    pub frame_count: Option<u64>,
    pub bit_depth: Option<u8>,            // e.g. 16, 24, 32
    pub compression_mode: Option<String>, // Lossy, Lossless
    pub stream_size: Option<u64>,
    pub delay_ms: Option<f64>,
    pub delay_relative_to_video_ms: Option<f64>,
    pub delay_source: Option<String>,
    pub dialnorm_db: Option<i32>, // e.g. -27 dB
    pub dolby_atmos_present: bool,
    pub title: Option<String>,
    pub language: Option<String>,
    pub language_full: Option<String>,
    pub default_flag: bool,
    pub forced_flag: bool,
    pub extra: HashMap<String, String>,
}

impl Default for AudioTrack {
    fn default() -> Self {
        Self {
            stream_id: 1,
            stream_order: None,
            format: AudioCodec::Other("Unknown".to_string()),
            format_info: None,
            format_profile: None,
            format_commercial: None,
            format_additional_features: None,
            codec_id: None,
            codec_id_info: None,
            duration_ms: None,
            bit_rate: None,
            bit_rate_mode: None,
            bit_rate_maximum: None,
            channels: 2,
            channel_layout: Some(AudioChannelLayout::Stereo),
            channel_positions: Some("2/0/0".to_string()),
            samples_per_frame: None,
            sampling_rate: 48000,
            sampling_count: None,
            frame_rate: None,
            frame_count: None,
            bit_depth: None,
            compression_mode: Some("Lossy".to_string()),
            stream_size: None,
            delay_ms: None,
            delay_relative_to_video_ms: None,
            delay_source: None,
            dialnorm_db: None,
            dolby_atmos_present: false,
            title: None,
            language: None,
            language_full: None,
            default_flag: true,
            forced_flag: false,
            extra: HashMap::new(),
        }
    }
}
