use crate::core::{error::Result, models::MediaReport};

/// Formats `MediaReport` into standard MediaInfo XML schema.
pub struct XmlFormatter;

impl XmlFormatter {
    pub fn format(report: &MediaReport) -> Result<String> {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<MediaInfo xmlns=\"https://mediaarea.net/mediainfo\" version=\"2.0\">\n");
        out.push_str(&format!(
            "  <media ref=\"{}\">\n",
            Self::escape_xml(&report.general.file_name.clone().unwrap_or_default())
        ));

        // General
        out.push_str("    <track type=\"General\">\n");
        Self::write_elem(&mut out, "Format", report.general.format.display_name(), 6);
        Self::write_elem(
            &mut out,
            "FileSize",
            &report.general.file_size.to_string(),
            6,
        );
        if let Some(dur) = report.general.duration_ms {
            Self::write_elem(&mut out, "Duration", &(dur / 1000.0).to_string(), 6);
        }
        if let Some(rate) = report.general.overall_bitrate {
            Self::write_elem(&mut out, "OverallBitRate", &rate.to_string(), 6);
        }
        if let Some(ref title) = report.general.title {
            Self::write_elem(&mut out, "Title", title, 6);
        }
        out.push_str("    </track>\n");

        // Videos
        for v in &report.videos {
            out.push_str("    <track type=\"Video\">\n");
            Self::write_elem(&mut out, "ID", &v.stream_id.to_string(), 6);
            Self::write_elem(&mut out, "Format", v.format.display_name(), 6);
            Self::write_elem(&mut out, "Width", &v.width.to_string(), 6);
            Self::write_elem(&mut out, "Height", &v.height.to_string(), 6);
            Self::write_elem(&mut out, "BitDepth", &v.bit_depth.to_string(), 6);
            if let Some(ref encoding) = v.color_encoding {
                Self::write_elem(&mut out, "ColorEncoding", encoding, 6);
            }
            if v.format == crate::core::types::VideoCodec::ProRes {
                for (key, value) in &v.extra {
                    Self::write_elem(&mut out, key, value, 6);
                }
            }
            out.push_str("    </track>\n");
        }

        // Audios
        for a in &report.audios {
            out.push_str("    <track type=\"Audio\">\n");
            Self::write_elem(&mut out, "ID", &a.stream_id.to_string(), 6);
            Self::write_elem(&mut out, "Format", a.format.display_name(), 6);
            Self::write_elem(&mut out, "Channels", &a.channels.to_string(), 6);
            Self::write_elem(&mut out, "SamplingRate", &a.sampling_rate.to_string(), 6);
            out.push_str("    </track>\n");
        }

        out.push_str("  </media>\n");
        out.push_str("</MediaInfo>\n");
        Ok(out)
    }

    fn write_elem(out: &mut String, tag: &str, val: &str, indent: usize) {
        let spaces = " ".repeat(indent);
        out.push_str(&format!(
            "{}<{}>{}</{}>\n",
            spaces,
            tag,
            Self::escape_xml(val),
            tag
        ));
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}
