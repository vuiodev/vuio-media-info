use serde::{Deserialize, Serialize};

/// Individual chapter / timestamp marker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub timestamp_ms: f64,
    pub title: String,
    pub language: Option<String>,
}

/// Menu / Chapters metadata track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuTrack {
    pub stream_id: u32,
    pub duration_ms: Option<f64>,
    pub chapters: Vec<Chapter>,
}

impl Default for MenuTrack {
    fn default() -> Self {
        Self {
            stream_id: 1,
            duration_ms: None,
            chapters: Vec::new(),
        }
    }
}
