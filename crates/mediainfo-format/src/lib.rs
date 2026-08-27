#![allow(clippy::field_reassign_with_default)]

pub mod csv;
pub mod html;
pub mod json;
pub mod text;
pub mod xml;

pub use csv::CsvFormatter;
pub use html::HtmlFormatter;
pub use json::JsonFormatter;
pub use text::TextFormatter;
pub use xml::XmlFormatter;

use mediainfo_core::{error::Result, models::MediaReport};

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Xml,
    Csv,
    Html,
}

impl OutputFormat {
    pub fn format(&self, report: &MediaReport) -> Result<String> {
        match self {
            Self::Text => Ok(TextFormatter::format(report)),
            Self::Json => JsonFormatter::format(report),
            Self::Xml => XmlFormatter::format(report),
            Self::Csv => CsvFormatter::format(report),
            Self::Html => HtmlFormatter::format(report),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mediainfo_core::models::*;
    use mediainfo_core::types::*;

    #[test]
    fn test_formatters() {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::Matroska;
        report.general.file_name = Some("sample.mkv".to_string());
        report.general.file_size = 1024 * 1024 * 50;
        report.general.duration_ms = Some(125000.0);

        let mut v = VideoTrack::default();
        v.stream_id = 1;
        v.format = VideoCodec::HEVC;
        v.width = 3840;
        v.height = 2160;
        v.bit_depth = 10;
        v.frame_rate = Some(23.976);
        report.videos.push(v);

        let mut a = AudioTrack::default();
        a.stream_id = 2;
        a.format = AudioCodec::EAC3;
        a.channels = 6;
        a.sampling_rate = 48000;
        report.audios.push(a);

        // Test Text
        let txt = OutputFormat::Text.format(&report).unwrap();
        assert!(txt.contains("General"));
        assert!(txt.contains("Matroska"));
        assert!(txt.contains("Video"));
        assert!(txt.contains("3 840 pixels"));

        // Test JSON
        let json_str = OutputFormat::Json.format(&report).unwrap();
        assert!(json_str.contains("\"@type\": \"General\""));
        assert!(json_str.contains("\"Width\": \"3840\""));

        // Test XML
        let xml_str = OutputFormat::Xml.format(&report).unwrap();
        assert!(xml_str.contains("<MediaInfo"));
        assert!(xml_str.contains("<track type=\"Video\">"));

        // Test CSV
        let csv_str = OutputFormat::Csv.format(&report).unwrap();
        assert!(csv_str.contains("sample.mkv"));

        // Test HTML
        let html_str = OutputFormat::Html.format(&report).unwrap();
        assert!(html_str.contains("<!DOCTYPE html>"));
        assert!(html_str.contains("MediaInfo Report"));
    }
}
