use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// Standalone Subtitle Files Demuxer (SRT, ASS, SSA, WebVTT, Bluray SUP).
pub struct SubtitleDemuxer;

pub const SUP_MAGIC: [u8; 2] = [0x50, 0x47]; // "PG"

impl SubtitleDemuxer {
    pub fn parse_buffer(data: &[u8], format: ContainerFormat) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = format;
        report.general.file_size = data.len() as u64;

        match format {
            ContainerFormat::SRT => Self::parse_srt(data, &mut report)?,
            ContainerFormat::ASS => Self::parse_ass(data, &mut report)?,
            ContainerFormat::WebVTT => Self::parse_vtt(data, &mut report)?,
            ContainerFormat::SUP => Self::parse_sup(data, &mut report)?,
            _ => {
                return Err(MediaInfoError::InvalidData(
                    "Unsupported subtitle container".to_string(),
                ));
            }
        }

        Ok(report)
    }

    fn parse_srt(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let (text, encoding) = Self::decode_text_with_bom(data);
        report.general.format_profile = Some(format!("SubRip ({})", encoding));

        let mut cue_count = 0u64;
        let mut max_timestamp_ms = 0.0f64;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("-->") {
                let parts: Vec<&str> = trimmed.split("-->").collect();
                if parts.len() == 2 {
                    cue_count += 1;
                    if let Some(end_ms) = Self::parse_srt_timestamp(parts[1].trim()) {
                        if end_ms > max_timestamp_ms {
                            max_timestamp_ms = end_ms;
                        }
                    }
                }
            }
        }

        let mut sub = TextTrack::default();
        sub.format = SubtitleCodec::SubRip;
        sub.format_info = Some(format!("SubRip Subtitle ({} cues)", cue_count));
        sub.element_count = Some(cue_count);

        if max_timestamp_ms > 0.0 {
            sub.duration_ms = Some(max_timestamp_ms);
            report.general.duration_ms = Some(max_timestamp_ms);
        }

        report.texts.push(sub);
        Ok(())
    }

    fn parse_ass(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let (text, encoding) = Self::decode_text_with_bom(data);
        let is_v4_plus = text.contains("[V4+ Styles]");
        report.general.format_profile = Some(format!(
            "{} ({})",
            if is_v4_plus {
                "Advanced SubStation Alpha (ASS v4+)"
            } else {
                "SubStation Alpha (SSA v4)"
            },
            encoding
        ));

        let mut dialogue_count = 0u64;
        let mut style_count = 0u64;
        let mut font_count = 0u64;
        let mut max_timestamp_ms = 0.0f64;

        let mut in_fonts = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_fonts = trimmed.eq_ignore_ascii_case("[Fonts]");
                continue;
            }

            if in_fonts && trimmed.starts_with("fontname:") {
                font_count += 1;
            }

            if trimmed.starts_with("Title:") {
                report.general.title = Some(trimmed["Title:".len()..].trim().to_string());
            } else if trimmed.starts_with("Original Script:") {
                report.general.artist =
                    Some(trimmed["Original Script:".len()..].trim().to_string());
            } else if trimmed.starts_with("Style:") {
                style_count += 1;
            } else if trimmed.starts_with("Dialogue:") {
                dialogue_count += 1;
                let payload = &trimmed["Dialogue:".len()..];
                let fields: Vec<&str> = payload.splitn(10, ',').collect();
                if fields.len() >= 3 {
                    if let Some(end_ms) = Self::parse_ass_timestamp(fields[2].trim()) {
                        if end_ms > max_timestamp_ms {
                            max_timestamp_ms = end_ms;
                        }
                    }
                }
            }
        }

        let mut sub = TextTrack::default();
        sub.format = if is_v4_plus {
            SubtitleCodec::ASS
        } else {
            SubtitleCodec::SSA
        };
        sub.format_info = Some(format!(
            "{} lines, {} styles, {} embedded fonts",
            dialogue_count, style_count, font_count
        ));
        sub.element_count = Some(dialogue_count);

        if max_timestamp_ms > 0.0 {
            sub.duration_ms = Some(max_timestamp_ms);
            report.general.duration_ms = Some(max_timestamp_ms);
        }

        report.texts.push(sub);
        Ok(())
    }

    fn parse_vtt(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let (text, encoding) = Self::decode_text_with_bom(data);
        report.general.format_profile = Some(format!("WebVTT ({})", encoding));

        let mut cue_count = 0u64;
        let mut max_timestamp_ms = 0.0f64;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("-->") {
                let parts: Vec<&str> = trimmed.split("-->").collect();
                if parts.len() == 2 {
                    cue_count += 1;
                    // Right side may contain cue settings: "00:04.000 line:0 position:20%"
                    let end_token = parts[1].trim().split_whitespace().next().unwrap_or("");
                    if let Some(end_ms) = Self::parse_vtt_timestamp(end_token) {
                        if end_ms > max_timestamp_ms {
                            max_timestamp_ms = end_ms;
                        }
                    }
                }
            }
        }

        let mut sub = TextTrack::default();
        sub.format = SubtitleCodec::WebVTT;
        sub.format_info = Some(format!("WebVTT Subtitle ({} cues)", cue_count));
        sub.element_count = Some(cue_count);

        if max_timestamp_ms > 0.0 {
            sub.duration_ms = Some(max_timestamp_ms);
            report.general.duration_ms = Some(max_timestamp_ms);
        }

        report.texts.push(sub);
        Ok(())
    }

    fn parse_sup(data: &[u8], report: &mut MediaReport) -> Result<()> {
        if data.len() < 13 || !data.starts_with(&SUP_MAGIC) {
            return Err(MediaInfoError::InvalidData(
                "Not a valid Blu-ray SUP file".to_string(),
            ));
        }

        report.general.format_profile =
            Some("Blu-ray Presentation Graphic Stream (PGS)".to_string());

        let mut segment_count = 0u64;
        let mut max_pts_ms = 0.0f64;
        let mut width = 1920u32;
        let mut height = 1080u32;

        let mut offset = 0;
        while offset + 13 <= data.len() {
            if &data[offset..offset + 2] != &SUP_MAGIC {
                offset += 1;
                continue;
            }

            let pts = u32::from_be_bytes([
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
            ]);
            let _dts = u32::from_be_bytes([
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
            ]);
            let seg_type = data[offset + 10];
            let seg_len = u16::from_be_bytes([data[offset + 11], data[offset + 12]]) as usize;
            offset += 13;

            segment_count += 1;
            let pts_ms = (pts as f64) / 90.0;
            if pts_ms > max_pts_ms {
                max_pts_ms = pts_ms;
            }

            if seg_type == 0x16 && offset + 4 <= data.len() && seg_len >= 4 {
                // Presentation Composition Segment (PCS)
                width = u16::from_be_bytes([data[offset], data[offset + 1]]) as u32;
                height = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as u32;
            }

            offset += seg_len;
        }

        let mut sub = TextTrack::default();
        sub.format = SubtitleCodec::PGS;
        sub.format_info = Some(format!(
            "HDMV PGS ({}x{}, {} segments)",
            width, height, segment_count
        ));
        sub.element_count = Some(segment_count);

        if max_pts_ms > 0.0 {
            sub.duration_ms = Some(max_pts_ms);
            report.general.duration_ms = Some(max_pts_ms);
        }

        report.texts.push(sub);
        Ok(())
    }

    fn decode_text_with_bom(data: &[u8]) -> (String, &'static str) {
        if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            (String::from_utf8_lossy(&data[3..]).to_string(), "UTF-8 BOM")
        } else if data.starts_with(&[0xFF, 0xFE]) {
            let u16_chars: Vec<u16> = data[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            (String::from_utf16_lossy(&u16_chars), "UTF-16LE")
        } else if data.starts_with(&[0xFE, 0xFF]) {
            let u16_chars: Vec<u16> = data[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            (String::from_utf16_lossy(&u16_chars), "UTF-16BE")
        } else {
            (String::from_utf8_lossy(data).to_string(), "UTF-8")
        }
    }

    fn parse_srt_timestamp(s: &str) -> Option<f64> {
        // "00:01:23,456"
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let s_ms: Vec<&str> = parts[2].split(|c| c == ',' || c == '.').collect();
            let sec = s_ms[0].parse::<f64>().ok()?;
            let ms = if s_ms.len() > 1 {
                s_ms[1].parse::<f64>().ok().unwrap_or(0.0)
            } else {
                0.0
            };
            Some((h * 3600.0 + m * 60.0 + sec) * 1000.0 + ms)
        } else {
            None
        }
    }

    fn parse_ass_timestamp(s: &str) -> Option<f64> {
        // "0:01:23.45"
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let s_cs: Vec<&str> = parts[2].split('.').collect();
            let sec = s_cs[0].parse::<f64>().ok()?;
            let cs = if s_cs.len() > 1 {
                s_cs[1].parse::<f64>().ok().unwrap_or(0.0)
            } else {
                0.0
            };
            Some((h * 3600.0 + m * 60.0 + sec) * 1000.0 + cs * 10.0)
        } else {
            None
        }
    }

    fn parse_vtt_timestamp(s: &str) -> Option<f64> {
        // "01:23.456" or "00:01:23.456"
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let s_ms: Vec<&str> = parts[2].split('.').collect();
            let sec = s_ms[0].parse::<f64>().ok()?;
            let ms = if s_ms.len() > 1 {
                s_ms[1].parse::<f64>().ok().unwrap_or(0.0)
            } else {
                0.0
            };
            Some((h * 3600.0 + m * 60.0 + sec) * 1000.0 + ms)
        } else if parts.len() == 2 {
            let m = parts[0].parse::<f64>().ok()?;
            let s_ms: Vec<&str> = parts[1].split('.').collect();
            let sec = s_ms[0].parse::<f64>().ok()?;
            let ms = if s_ms.len() > 1 {
                s_ms[1].parse::<f64>().ok().unwrap_or(0.0)
            } else {
                0.0
            };
            Some((m * 60.0 + sec) * 1000.0 + ms)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_subtitle_parser() {
        let srt_data = b"1\n00:00:01,000 --> 00:00:04,500\nHello World!\n\n2\n00:00:05,000 --> 00:00:10,000\nGoodbye!\n";
        let report = SubtitleDemuxer::parse_buffer(srt_data, ContainerFormat::SRT).unwrap();
        assert_eq!(report.general.format, ContainerFormat::SRT);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.texts.len(), 1);
        assert_eq!(report.texts[0].element_count, Some(2));
    }

    #[test]
    fn test_ass_subtitle_parser() {
        let ass_data = b"[Script Info]\nTitle: Test Episode\n[V4+ Styles]\nStyle: Default\n[Events]\nDialogue: 0,0:00:01.00,0:00:05.50,Default,,0,0,0,,Hello!\n";
        let report = SubtitleDemuxer::parse_buffer(ass_data, ContainerFormat::ASS).unwrap();
        assert_eq!(report.general.format, ContainerFormat::ASS);
        assert_eq!(report.general.title, Some("Test Episode".to_string()));
        assert_eq!(report.general.duration_ms, Some(5500.0));
        assert_eq!(report.texts[0].element_count, Some(1));
    }

    #[test]
    fn test_vtt_subtitle_parser() {
        let vtt_data = b"WEBVTT\n\n00:01.000 --> 00:05.000\nSubtitle text\n";
        let report = SubtitleDemuxer::parse_buffer(vtt_data, ContainerFormat::WebVTT).unwrap();
        assert_eq!(report.general.format, ContainerFormat::WebVTT);
        assert_eq!(report.general.duration_ms, Some(5000.0));
    }
}
