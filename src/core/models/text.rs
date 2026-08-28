use crate::core::types::SubtitleCodec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Subtitle / Text stream metadata track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextTrack {
    pub stream_id: u32,
    pub stream_order: Option<u32>,
    pub format: SubtitleCodec,
    pub format_info: Option<String>,
    pub format_profile: Option<String>,
    pub codec_id: Option<String>,
    pub codec_id_info: Option<String>,
    pub duration_ms: Option<f64>,
    pub bit_rate: Option<u64>,
    pub frame_rate: Option<f64>,
    pub frame_count: Option<u64>,
    pub element_count: Option<u64>,
    pub stream_size: Option<u64>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub language_full: Option<String>,
    pub default_flag: bool,
    pub forced_flag: bool,
    pub hearing_impaired: bool,
    pub extra: HashMap<String, String>,
}

impl Default for TextTrack {
    fn default() -> Self {
        Self {
            stream_id: 1,
            stream_order: None,
            format: SubtitleCodec::Other("Unknown".to_string()),
            format_info: None,
            format_profile: None,
            codec_id: None,
            codec_id_info: None,
            duration_ms: None,
            bit_rate: None,
            frame_rate: None,
            frame_count: None,
            element_count: None,
            stream_size: None,
            title: None,
            language: None,
            language_full: None,
            default_flag: false,
            forced_flag: false,
            hearing_impaired: false,
            extra: HashMap::new(),
        }
    }
}
