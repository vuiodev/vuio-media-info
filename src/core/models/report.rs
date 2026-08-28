use super::{
    attachment::Attachment, audio::AudioTrack, bitstream_node::BitstreamNode,
    general::GeneralTrack, menu::MenuTrack, text::TextTrack, video::VideoTrack,
};
use serde::{Deserialize, Serialize};

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
    /// Create a new empty `MediaReport`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total count of all tracks (general + video + audio + text + menu).
    pub fn track_count(&self) -> usize {
        1 + self.videos.len() + self.audios.len() + self.texts.len() + self.menus.len()
    }

    /// Check if report contains no media tracks and zero file size.
    pub fn is_empty(&self) -> bool {
        self.videos.is_empty() && self.audios.is_empty() && self.general.file_size == 0
    }

    /// Convenience getter for the primary video track, if present.
    pub fn primary_video(&self) -> Option<&VideoTrack> {
        self.videos.first()
    }

    /// Convenience getter for the primary audio track, if present.
    pub fn primary_audio(&self) -> Option<&AudioTrack> {
        self.audios.first()
    }

    /// Format this report using a specified `OutputFormat`.
    pub fn format(
        &self,
        output_format: crate::format::OutputFormat,
    ) -> crate::core::error::Result<String> {
        output_format.format(self)
    }

    /// Format this report as official MediaInfo JSON schema string.
    pub fn to_json(&self) -> crate::core::error::Result<String> {
        crate::format::OutputFormat::Json.format(self)
    }

    /// Format this report as 2-column aligned classic MediaInfo text.
    pub fn to_text(&self) -> crate::core::error::Result<String> {
        crate::format::OutputFormat::Text.format(self)
    }

    /// Format this report as MediaInfo 2.0 XML.
    pub fn to_xml(&self) -> crate::core::error::Result<String> {
        crate::format::OutputFormat::Xml.format(self)
    }

    /// Format this report as a single-row CSV summary.
    pub fn to_csv(&self) -> crate::core::error::Result<String> {
        crate::format::OutputFormat::Csv.format(self)
    }

    /// Format this report as a standalone styled HTML document.
    pub fn to_html(&self) -> crate::core::error::Result<String> {
        crate::format::OutputFormat::Html.format(self)
    }
}
