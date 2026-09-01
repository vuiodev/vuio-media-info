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
            let mut put = |key: &str, val: Option<String>| {
                if let Some(val) = val {
                    v_val[key] = json!(val);
                }
            };
            put("Format_Info", v.format_info.clone());
            put("Format_Profile", v.format_profile.clone());
            put("Format_Version", v.format_version.clone());
            put("Format_Level", v.format_level.clone());
            put("Format_Tier", v.format_tier.clone());
            put("Format_Commercial", v.format_commercial.clone());
            put("CodecID", v.codec_id.clone());
            put("CodecID_Info", v.codec_id_info.clone());
            put("Duration", v.duration_ms.map(|d| (d / 1000.0).to_string()));
            put("BitRate", v.bit_rate.map(|b| b.to_string()));
            put(
                "BitRate_Mode",
                v.bit_rate_mode.map(|m| m.display_name().to_string()),
            );
            put("BitRate_Maximum", v.bit_rate_maximum.map(|b| b.to_string()));
            put("Stored_Width", v.stored_width.map(|w| w.to_string()));
            put("Stored_Height", v.stored_height.map(|h| h.to_string()));
            put(
                "PixelAspectRatio",
                v.sample_aspect_ratio.map(|r| format!("{r:.3}")),
            );
            put(
                "DisplayAspectRatio",
                v.display_aspect_ratio.map(|r| format!("{r:.3}")),
            );
            put(
                "FrameRate_Mode",
                v.frame_rate_mode.map(|m| m.display_name().to_string()),
            );
            put("FrameRate", v.frame_rate.map(|f| format!("{f:.3}")));
            put("FrameCount", v.frame_count.map(|c| c.to_string()));
            put("Standard", v.standard.clone());
            put("ColorSpace", v.color_space.clone());
            put("ColorEncoding", v.color_encoding.clone());
            put(
                "ChromaSubsampling",
                v.chroma_subsampling.map(|c| c.display_name().to_string()),
            );
            put("ScanType", v.scan_type.clone());
            put("ScanOrder", v.scan_order.clone());
            put("Compression_Mode", v.compression_mode.clone());
            put("StreamSize", v.stream_size.map(|s| s.to_string()));
            put("Encoded_Library", v.encoded_library.clone());
            put("Encoded_Library_Name", v.encoded_library_name.clone());
            put("Encoded_Library_Version", v.encoded_library_version.clone());
            put(
                "Encoded_Library_Settings",
                v.encoded_library_settings.clone(),
            );
            put(
                "colour_range",
                v.color_range.map(|r| r.display_name().to_string()),
            );
            // An "Unspecified" colour tag carries no information, so it is left out
            // rather than reported as if it were a real value.
            put(
                "colour_primaries",
                v.color_primaries
                    .map(|c| c.display_name().to_string())
                    .filter(|n| n != "Unspecified"),
            );
            put(
                "transfer_characteristics",
                v.transfer_characteristics
                    .map(|t| t.display_name().to_string())
                    .filter(|n| n != "Unspecified"),
            );
            put(
                "matrix_coefficients",
                v.matrix_coefficients
                    .map(|m| m.display_name().to_string())
                    .filter(|n| n != "Unspecified"),
            );
            put("HDR_Format", v.hdr_format.clone());
            put(
                "HDR_Format_Compatibility",
                v.hdr_format_compatibility.clone(),
            );
            put("Delay", v.delay_ms.map(|d| (d / 1000.0).to_string()));
            put("Title", v.title.clone());
            put("Language", v.language.clone());
            v_val["Default"] = json!(if v.default_flag { "Yes" } else { "No" });
            v_val["Forced"] = json!(if v.forced_flag { "Yes" } else { "No" });

            if let Some(ref dv) = v.dolby_vision {
                v_val["DolbyVision_Profile"] = json!(dv.profile.display_name());
                v_val["DolbyVision_Level"] = json!(dv.level.to_string());
            }
            for (k, val) in &v.extra {
                v_val[k.as_str()] = json!(val);
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
            let mut put = |key: &str, val: Option<String>| {
                if let Some(val) = val {
                    a_val[key] = json!(val);
                }
            };
            put("Format_Info", a.format_info.clone());
            put(
                "Format_Profile",
                a.format_profile
                    .clone()
                    .or_else(|| a.format.format_profile().map(str::to_string)),
            );
            put("Format_Commercial", a.format_commercial.clone());
            put(
                "Format_AdditionalFeatures",
                a.format_additional_features.clone(),
            );
            put("CodecID", a.codec_id.clone());
            put("CodecID_Info", a.codec_id_info.clone());
            put("Duration", a.duration_ms.map(|d| (d / 1000.0).to_string()));
            put("BitRate", a.bit_rate.map(|b| b.to_string()));
            put(
                "BitRate_Mode",
                a.bit_rate_mode.map(|m| m.display_name().to_string()),
            );
            put("BitRate_Maximum", a.bit_rate_maximum.map(|b| b.to_string()));
            put(
                "ChannelLayout",
                a.channel_layout
                    .as_ref()
                    .map(|l| l.display_name().to_string()),
            );
            put("ChannelPositions", a.channel_positions.clone());
            put(
                "SamplesPerFrame",
                a.samples_per_frame.map(|s| s.to_string()),
            );
            put("SamplingCount", a.sampling_count.map(|s| s.to_string()));
            put("FrameRate", a.frame_rate.map(|f| format!("{f:.3}")));
            put("FrameCount", a.frame_count.map(|c| c.to_string()));
            put("BitDepth", a.bit_depth.map(|d| d.to_string()));
            put("Compression_Mode", a.compression_mode.clone());
            put("StreamSize", a.stream_size.map(|s| s.to_string()));
            put("Delay", a.delay_ms.map(|d| (d / 1000.0).to_string()));
            put("Dialnorm", a.dialnorm_db.map(|d| d.to_string()));
            put("Title", a.title.clone());
            put("Language", a.language.clone());
            if a.dolby_atmos_present {
                a_val["Format_AdditionalFeatures"] = json!("Dolby Atmos");
            }
            a_val["Default"] = json!(if a.default_flag { "Yes" } else { "No" });
            a_val["Forced"] = json!(if a.forced_flag { "Yes" } else { "No" });
            for (k, val) in &a.extra {
                a_val[k.as_str()] = json!(val);
            }
            tracks.push(a_val);
        }

        // Subtitle tracks
        for t in &report.texts {
            let mut t_val = json!({
                "@type": "Text",
                "StreamOrder": t.stream_id.to_string(),
                "ID": t.stream_id.to_string(),
                "Format": t.format.display_name(),
            });
            let mut put = |key: &str, val: Option<String>| {
                if let Some(val) = val {
                    t_val[key] = json!(val);
                }
            };
            put("CodecID", t.codec_id.clone());
            put("Duration", t.duration_ms.map(|d| (d / 1000.0).to_string()));
            put("Title", t.title.clone());
            put("Language", t.language.clone());
            t_val["Default"] = json!(if t.default_flag { "Yes" } else { "No" });
            t_val["Forced"] = json!(if t.forced_flag { "Yes" } else { "No" });
            tracks.push(t_val);
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
