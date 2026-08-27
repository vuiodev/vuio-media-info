use mediainfo_core::models::MediaReport;

/// Formats `MediaReport` into classic MediaInfo aligned key-value text.
pub struct TextFormatter;

impl TextFormatter {
    pub fn format(report: &MediaReport) -> String {
        let mut out = String::new();

        // 1. General Track
        out.push_str("General\n");
        if let Some(ref name) = report.general.file_name {
            Self::write_line(&mut out, "Complete name", name);
        }
        Self::write_line(&mut out, "Format", report.general.format.display_name());
        if let Some(ref profile) = report.general.format_profile {
            Self::write_line(&mut out, "Format profile", profile);
        }
        if let Some(ref ver) = report.general.format_version {
            Self::write_line(&mut out, "Format version", ver);
        }
        if let Some(ref cid) = report.general.codec_id {
            Self::write_line(&mut out, "Codec ID", cid);
        }
        if report.general.file_size > 0 {
            Self::write_line(&mut out, "File size", &Self::format_file_size(report.general.file_size));
        }
        if let Some(dur_ms) = report.general.duration_ms {
            Self::write_line(&mut out, "Duration", &Self::format_duration(dur_ms));
        }
        if let Some(rate) = report.general.overall_bitrate {
            Self::write_line(&mut out, "Overall bit rate", &Self::format_bitrate(rate));
        }
        if let Some(ref title) = report.general.title {
            Self::write_line(&mut out, "Movie name / Title", title);
        }
        if let Some(ref artist) = report.general.artist {
            Self::write_line(&mut out, "Performer / Artist", artist);
        }
        if let Some(ref album) = report.general.album {
            Self::write_line(&mut out, "Album", album);
        }
        if let Some(ref genre) = report.general.genre {
            Self::write_line(&mut out, "Genre", genre);
        }
        if let Some(ref date) = report.general.recorded_date {
            Self::write_line(&mut out, "Recorded date", date);
        }
        if let Some(ref app) = report.general.encoded_application {
            Self::write_line(&mut out, "Writing application", app);
        }
        if let Some(ref lib) = report.general.encoded_library {
            Self::write_line(&mut out, "Writing library", lib);
        }
        if report.general.cover_art_present {
            Self::write_line(&mut out, "Cover art", "Yes");
        }
        out.push('\n');

        // 2. Video Tracks
        for (i, v) in report.videos.iter().enumerate() {
            if report.videos.len() > 1 {
                out.push_str(&format!("Video #{}\n", i + 1));
            } else {
                out.push_str("Video\n");
            }

            Self::write_line(&mut out, "ID", &v.stream_id.to_string());
            Self::write_line(&mut out, "Format", v.format.display_name());
            if let Some(ref info) = v.format_info {
                Self::write_line(&mut out, "Format/Info", info);
            }
            if let Some(ref profile) = v.format_profile {
                let profile_str = if let Some(ref level) = v.format_level {
                    format!("{}@L{}", profile, level)
                } else {
                    profile.clone()
                };
                Self::write_line(&mut out, "Format profile", &profile_str);
            }
            if let Some(ref hdr) = v.hdr_format {
                Self::write_line(&mut out, "HDR format", hdr);
            }
            if let Some(ref cid) = v.codec_id {
                Self::write_line(&mut out, "Codec ID", cid);
            }
            if let Some(dur_ms) = v.duration_ms.or(report.general.duration_ms) {
                Self::write_line(&mut out, "Duration", &Self::format_duration(dur_ms));
            }
            if let Some(bitrate) = v.bit_rate {
                Self::write_line(&mut out, "Bit rate", &Self::format_bitrate(bitrate));
            }
            if v.width > 0 && v.height > 0 {
                Self::write_line(&mut out, "Width", &format!("{} pixels", Self::format_number(v.width as u64)));
                Self::write_line(&mut out, "Height", &format!("{} pixels", Self::format_number(v.height as u64)));
                let dar = (v.width as f64) / (v.height as f64);
                if (dar - 16.0 / 9.0).abs() < 0.05 {
                    Self::write_line(&mut out, "Display aspect ratio", "16:9");
                } else if (dar - 4.0 / 3.0).abs() < 0.05 {
                    Self::write_line(&mut out, "Display aspect ratio", "4:3");
                } else {
                    Self::write_line(&mut out, "Display aspect ratio", &format!("{:.3}", dar));
                }
            }
            if let Some(mode) = v.frame_rate_mode {
                Self::write_line(&mut out, "Frame rate mode", mode.display_name());
            }
            if let Some(fps) = v.frame_rate {
                Self::write_line(&mut out, "Frame rate", &format!("{:.3} FPS", fps));
            }
            if let Some(ref cs) = v.color_space {
                Self::write_line(&mut out, "Color space", cs);
            }
            if let Some(sub) = v.chroma_subsampling {
                Self::write_line(&mut out, "Chroma subsampling", sub.display_name());
            }
            Self::write_line(&mut out, "Bit depth", &format!("{} bits", v.bit_depth));
            if let Some(range) = v.color_range {
                Self::write_line(&mut out, "Color range", range.display_name());
            }
            if let Some(prim) = v.color_primaries {
                Self::write_line(&mut out, "Color primaries", prim.display_name());
            }
            if let Some(tc) = v.transfer_characteristics {
                Self::write_line(&mut out, "Transfer characteristics", tc.display_name());
            }
            if let Some(mc) = v.matrix_coefficients {
                Self::write_line(&mut out, "Matrix coefficients", mc.display_name());
            }
            if let Some(ref title) = v.title {
                Self::write_line(&mut out, "Title", title);
            }
            if let Some(ref lang) = v.language {
                Self::write_line(&mut out, "Language", lang);
            }
            Self::write_line(&mut out, "Default", if v.default_flag { "Yes" } else { "No" });
            Self::write_line(&mut out, "Forced", if v.forced_flag { "Yes" } else { "No" });
            out.push('\n');
        }

        // 3. Audio Tracks
        for (i, a) in report.audios.iter().enumerate() {
            if report.audios.len() > 1 {
                out.push_str(&format!("Audio #{}\n", i + 1));
            } else {
                out.push_str("Audio\n");
            }

            Self::write_line(&mut out, "ID", &a.stream_id.to_string());
            let format_str = if a.dolby_atmos_present {
                format!("{} JOC / Atmos", a.format.display_name())
            } else {
                a.format.display_name().to_string()
            };
            Self::write_line(&mut out, "Format", &format_str);
            if let Some(ref info) = a.format_info {
                Self::write_line(&mut out, "Format/Info", info);
            }
            if let Some(ref profile) = a.format_profile {
                Self::write_line(&mut out, "Format profile", profile);
            }
            if let Some(ref cid) = a.codec_id {
                Self::write_line(&mut out, "Codec ID", cid);
            }
            if let Some(dur_ms) = a.duration_ms.or(report.general.duration_ms) {
                Self::write_line(&mut out, "Duration", &Self::format_duration(dur_ms));
            }
            if let Some(bitrate) = a.bit_rate {
                Self::write_line(&mut out, "Bit rate", &Self::format_bitrate(bitrate));
            }
            Self::write_line(&mut out, "Channel(s)", &format!("{} channels", a.channels));
            if let Some(ref layout) = a.channel_layout {
                Self::write_line(&mut out, "Channel layout", layout.display_name());
            }
            Self::write_line(&mut out, "Sampling rate", &format!("{:.1} kHz", a.sampling_rate as f64 / 1000.0));
            if let Some(depth) = a.bit_depth {
                Self::write_line(&mut out, "Bit depth", &format!("{} bits", depth));
            }
            if let Some(ref comp) = a.compression_mode {
                Self::write_line(&mut out, "Compression mode", comp);
            }
            if let Some(dialnorm) = a.dialnorm_db {
                Self::write_line(&mut out, "Dialog Normalization", &format!("{} dB", dialnorm));
            }
            if let Some(ref title) = a.title {
                Self::write_line(&mut out, "Title", title);
            }
            if let Some(ref lang) = a.language {
                Self::write_line(&mut out, "Language", lang);
            }
            Self::write_line(&mut out, "Default", if a.default_flag { "Yes" } else { "No" });
            Self::write_line(&mut out, "Forced", if a.forced_flag { "Yes" } else { "No" });
            out.push('\n');
        }

        // 4. Subtitles
        for (i, s) in report.texts.iter().enumerate() {
            if report.texts.len() > 1 {
                out.push_str(&format!("Text #{}\n", i + 1));
            } else {
                out.push_str("Text\n");
            }
            Self::write_line(&mut out, "ID", &s.stream_id.to_string());
            Self::write_line(&mut out, "Format", s.format.display_name());
            if let Some(ref info) = s.format_info {
                Self::write_line(&mut out, "Format/Info", info);
            }
            if let Some(ref cid) = s.codec_id {
                Self::write_line(&mut out, "Codec ID", cid);
            }
            if let Some(ref title) = s.title {
                Self::write_line(&mut out, "Title", title);
            }
            if let Some(ref lang) = s.language {
                Self::write_line(&mut out, "Language", lang);
            }
            Self::write_line(&mut out, "Default", if s.default_flag { "Yes" } else { "No" });
            Self::write_line(&mut out, "Forced", if s.forced_flag { "Yes" } else { "No" });
            out.push('\n');
        }

        // 5. Chapters / Menu
        for (_i, m) in report.menus.iter().enumerate() {
            out.push_str("Menu\n");
            for chap in &m.chapters {
                let ms = chap.timestamp_ms;
                let total_sec = (ms / 1000.0) as u64;
                let h = total_sec / 3600;
                let min = (total_sec % 3600) / 60;
                let s = total_sec % 60;
                let frac = (ms % 1000.0) as u64;
                let time_str = format!("{:02}:{:02}:{:02}.{:03}", h, min, s, frac);
                Self::write_line(&mut out, &time_str, &chap.title);
            }
            out.push('\n');
        }

        out
    }

    fn write_line(out: &mut String, key: &str, value: &str) {
        out.push_str(&format!("{:<40} : {}\n", key, value));
    }

    fn format_file_size(bytes: u64) -> String {
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
        const MIB: f64 = 1024.0 * 1024.0;
        const KIB: f64 = 1024.0;

        if bytes as f64 >= GIB {
            format!("{:.2} GiB", bytes as f64 / GIB)
        } else if bytes as f64 >= MIB {
            format!("{:.2} MiB", bytes as f64 / MIB)
        } else if bytes as f64 >= KIB {
            format!("{:.2} KiB", bytes as f64 / KIB)
        } else {
            format!("{} Bytes", bytes)
        }
    }

    fn format_duration(ms: f64) -> String {
        let total_secs = (ms / 1000.0) as u64;
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;

        if hours > 0 {
            format!("{} h {} min", hours, mins)
        } else if mins > 0 {
            format!("{} min {} s", mins, secs)
        } else {
            format!("{} s", secs)
        }
    }

    fn format_bitrate(bps: u64) -> String {
        if bps >= 1_000_000 {
            format!("{} kb/s", Self::format_number(bps / 1000))
        } else {
            format!("{} b/s", Self::format_number(bps))
        }
    }

    fn format_number(n: u64) -> String {
        let s = n.to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(' ');
            }
            result.push(c);
        }
        result
    }
}
