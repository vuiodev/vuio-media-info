use crate::core::{
    error::{MediaInfoError, Result},
    models::MediaReport,
};
use serde_json::json;

/// Formats `MediaReport` into standard MediaInfo JSON schema.
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn format(report: &MediaReport) -> Result<String> {
        let mut tracks = Vec::new();

        // General track
        let mut gen_val = json!({
            "@type": "General",
            "Format": report.general.format.display_name(),
            "FileSize": report.general.file_size.to_string(),
        });

        if let Some(ref name) = report.general.file_name {
            gen_val["CompleteName"] = json!(name);
        }
        if let Some(ref profile) = report.general.format_profile {
            gen_val["Format_Profile"] = json!(profile);
        }
        if let Some(ref ver) = report.general.format_version {
            gen_val["Format_Version"] = json!(ver);
        }
        if let Some(dur) = report.general.duration_ms {
            gen_val["Duration"] = json!((dur / 1000.0).to_string());
        }
        if let Some(rate) = report.general.overall_bitrate {
            gen_val["OverallBitRate"] = json!(rate.to_string());
        }
        if let Some(ref title) = report.general.title {
            gen_val["Title"] = json!(title);
        }
        if let Some(ref artist) = report.general.artist {
            gen_val["Performer"] = json!(artist);
        }
        if let Some(ref album) = report.general.album {
            gen_val["Album"] = json!(album);
        }
        if let Some(ref app) = report.general.encoded_application {
            gen_val["Encoded_Application"] = json!(app);
        }
        if let Some(ref lib) = report.general.encoded_library {
            gen_val["Encoded_Library"] = json!(lib);
        }

        tracks.push(gen_val);

        // Video tracks
        for v in &report.videos {
            let mut v_val = json!({
                "@type": "Video",
                "StreamOrder": v.stream_id.to_string(),
                "ID": v.stream_id.to_string(),
                "Format": v.format.display_name(),
                "Width": v.width.to_string(),
                "Height": v.height.to_string(),
                "BitDepth": v.bit_depth.to_string(),
            });
            if let Some(ref info) = v.format_info {
                v_val["Format_Info"] = json!(info);
            }
            if let Some(ref profile) = v.format_profile {
                v_val["Format_Profile"] = json!(profile);
            }
            if let Some(ref hdr) = v.hdr_format {
                v_val["HDR_Format"] = json!(hdr);
            }
            if let Some(fps) = v.frame_rate {
                v_val["FrameRate"] = json!(fps.to_string());
            }
            if let Some(sub) = v.chroma_subsampling {
                v_val["ChromaSubsampling"] = json!(sub.display_name());
            }
            if let Some(cp) = v.color_primaries {
                v_val["colour_primaries"] = json!(cp.display_name());
            }
            if let Some(tc) = v.transfer_characteristics {
                v_val["transfer_characteristics"] = json!(tc.display_name());
            }
            if let Some(mc) = v.matrix_coefficients {
                v_val["matrix_coefficients"] = json!(mc.display_name());
            }
            tracks.push(v_val);
        }

        // Audio tracks
        for a in &report.audios {
            let mut a_val = json!({
                "@type": "Audio",
                "StreamOrder": a.stream_id.to_string(),
                "ID": a.stream_id.to_string(),
                "Format": a.format.display_name(),
                "Channels": a.channels.to_string(),
                "SamplingRate": a.sampling_rate.to_string(),
            });
            if let Some(ref info) = a.format_info {
                a_val["Format_Info"] = json!(info);
            }
            if let Some(ref layout) = a.channel_layout {
                a_val["ChannelLayout"] = json!(layout.display_name());
            }
            if let Some(depth) = a.bit_depth {
                a_val["BitDepth"] = json!(depth.to_string());
            }
            if let Some(bitrate) = a.bit_rate {
                a_val["BitRate"] = json!(bitrate.to_string());
            }
            if let Some(ref lang) = a.language {
                a_val["Language"] = json!(lang);
            }
            if let Some(ref title) = a.title {
                a_val["Title"] = json!(title);
            }
            tracks.push(a_val);
        }

        // Subtitle tracks
        for s in &report.texts {
            let mut s_val = json!({
                "@type": "Text",
                "StreamOrder": s.stream_id.to_string(),
                "ID": s.stream_id.to_string(),
                "Format": s.format.display_name(),
            });
            if let Some(ref lang) = s.language {
                s_val["Language"] = json!(lang);
            }
            if let Some(ref title) = s.title {
                s_val["Title"] = json!(title);
            }
            tracks.push(s_val);
        }

        let root = json!({
            "media": {
                "@ref": report.general.file_name.clone().unwrap_or_default(),
                "track": tracks,
            }
        });

        serde_json::to_string_pretty(&root).map_err(MediaInfoError::from)
    }
}
