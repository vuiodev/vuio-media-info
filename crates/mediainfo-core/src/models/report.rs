use serde::{Deserialize, Serialize};
use crate::models::{
    attachment::Attachment,
    audio::AudioTrack,
    bitstream_node::BitstreamNode,
    general::GeneralTrack,
    menu::MenuTrack,
    text::TextTrack,
    video::VideoTrack,
};

/// Root media inspection report containing all parsed tracks and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaReport {
    pub version: String,
    pub general: GeneralTrack,
    pub videos: Vec<VideoTrack>,
    pub audios: Vec<AudioTrack>,
    pub texts: Vec<TextTrack>,
    pub menus: Vec<MenuTrack>,
    pub attachments: Vec<Attachment>,
    pub bitstream_root: Option<BitstreamNode>,
}

impl Default for MediaReport {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            general: GeneralTrack::default(),
            videos: Vec::new(),
            audios: Vec::new(),
            texts: Vec::new(),
            menus: Vec::new(),
            attachments: Vec::new(),
            bitstream_root: None,
        }
    }
}

impl MediaReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track_count(&self) -> usize {
        1 + self.videos.len() + self.audios.len() + self.texts.len() + self.menus.len()
    }

    pub fn primary_video(&self) -> Option<&VideoTrack> {
        self.videos.first()
    }

    pub fn primary_audio(&self) -> Option<&AudioTrack> {
        self.audios.first()
    }
}
