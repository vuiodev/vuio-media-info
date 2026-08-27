use mediainfo_audio::{AacInfo, FlacStreamInfo, OpusHead};
use mediainfo_core::{
    bitstream::EbmlVint,
    error::Result,
    models::*,
    types::*,
};
use mediainfo_video::{AvcSps, HevcSps};

/// Matroska (MKV) and WebM EBML container demuxer.
pub struct MatroskaDemuxer;

impl MatroskaDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::Matroska;
        report.general.file_size = data.len() as u64;

        let root_node = BitstreamNode::new("EBML", 0, data.len() as u64);
        let mut offset = 0;

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };

            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };

            let header_len = id_len + size_len;
            let element_size = size_opt.unwrap_or(data.len() as u64 - (offset + header_len) as u64) as usize;
            let payload_offset = offset + header_len;
            let payload = if payload_offset + element_size <= data.len() {
                &data[payload_offset..payload_offset + element_size]
            } else {
                &data[payload_offset..]
            };

            match id {
                0x1A45DFA3 => {
                    // EBML Header
                    Self::parse_ebml_header(payload, &mut report);
                }
                0x18538067 => {
                    // Segment
                    Self::parse_segment(payload, &mut report);
                    if !report.videos.is_empty() || !report.audios.is_empty() {
                        break;
                    }
                }
                _ => {}
            }

            offset += header_len + element_size;
        }

        report.bitstream_root = Some(root_node);
        Ok(report)
    }

    fn parse_ebml_header(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0x4282 {
                // DocType
                let doctype = String::from_utf8_lossy(payload).trim().to_string();
                if doctype.eq_ignore_ascii_case("webm") {
                    report.general.format = ContainerFormat::WebM;
                } else {
                    report.general.format = ContainerFormat::Matroska;
                }
            }

            offset = payload_off + size;
        }
    }

    fn parse_segment(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        let mut timecode_scale = 1_000_000u64; // Default 1ms

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0x1549A966 => {
                    // Info
                    Self::parse_info(payload, &mut timecode_scale, report);
                }
                0x1654AE6B => {
                    // Tracks
                    Self::parse_tracks(payload, timecode_scale, report);
                    if report.general.duration_ms.is_some() {
                        // We already have Info + Tracks!
                        break;
                    }
                }
                0x1043A770 => {
                    // Chapters
                    Self::parse_chapters(payload, report);
                }
                0x1941A469 => {
                    // Attachments
                    Self::parse_attachments(payload, report);
                }
                0x1254C367 => {
                    // Tags
                    Self::parse_tags(payload, report);
                }
                0x1F43B675 | 0xA3 | 0xA0 => {
                    // Cluster / Block
                    if !report.videos.is_empty() || !report.audios.is_empty() {
                        break;
                    }
                }
                _ => {
                    if size_opt.is_none() && (!report.videos.is_empty() || !report.audios.is_empty()) {
                        break;
                    }
                }
            }

            offset = payload_off + size;
        }
    }

    fn parse_info(data: &[u8], timecode_scale: &mut u64, report: &mut MediaReport) {
        let mut offset = 0;
        let mut duration_float = 0.0f64;

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0x2AD7B1 => {
                    // TimecodeScale (1–8 bytes big-endian)
                    if size >= 1 && size <= 8 {
                        let mut val = 0u64;
                        for &b in payload.iter().take(size) {
                            val = (val << 8) | b as u64;
                        }
                        if val > 0 {
                            *timecode_scale = val;
                        }
                    }
                }
                0x4489 => {
                    // Duration (float)
                    if size == 4 {
                        duration_float = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64;
                    } else if size == 8 {
                        duration_float = f64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6], payload[7],
                        ]);
                    }
                }
                0x7BA9 => {
                    // Title
                    report.general.title = Some(String::from_utf8_lossy(payload).to_string());
                }
                0x4D80 => {
                    // MuxingApp
                    report.general.encoded_application = Some(String::from_utf8_lossy(payload).to_string());
                }
                0x5741 => {
                    // WritingApp
                    report.general.encoded_library = Some(String::from_utf8_lossy(payload).to_string());
                }
                _ => {}
            }

            offset = payload_off + size;
        }

        if duration_float > 0.0 {
            let duration_ms = (duration_float * *timecode_scale as f64) / 1_000_000.0;
            report.general.duration_ms = Some(duration_ms);
            if duration_ms > 0.0 && report.general.file_size > 0 {
                report.general.overall_bitrate =
                    Some(((report.general.file_size * 8) as f64 / (duration_ms / 1000.0)) as u64);
            }
        }
    }

    fn parse_tracks(data: &[u8], timecode_scale: u64, report: &mut MediaReport) {
        let mut offset = 0;
        let mut track_id = 1u32;

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0xAE {
                // TrackEntry
                Self::parse_track_entry(payload, track_id, timecode_scale, report);
                track_id += 1;
            }

            offset = payload_off + size;
        }
    }

    fn parse_track_entry(data: &[u8], stream_id: u32, _timecode_scale: u64, report: &mut MediaReport) {
        let mut offset = 0;
        let mut track_type = 0u8;
        let mut codec_id = String::new();
        let mut codec_private = Vec::new();
        let mut name = None;
        let mut language = None;
        let mut default_flag = true;
        let mut forced_flag = false;
        let mut default_duration_ns = 0u64;

        let mut video_payload = None;
        let mut audio_payload = None;

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0x83 => {
                    track_type = payload.first().copied().unwrap_or(0);
                }
                0x86 => {
                    codec_id = String::from_utf8_lossy(payload).trim().to_string();
                }
                0x63A2 => {
                    codec_private = payload.to_vec();
                }
                0x536E => {
                    name = Some(String::from_utf8_lossy(payload).to_string());
                }
                0x22B59C => {
                    language = Some(String::from_utf8_lossy(payload).to_string());
                }
                0x88 => {
                    default_flag = payload.first().copied().unwrap_or(1) != 0;
                }
                0x55AA => {
                    forced_flag = payload.first().copied().unwrap_or(0) != 0;
                }
                0x23E383 => {
                    if size == 4 {
                        default_duration_ns = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as u64;
                    } else if size == 8 {
                        default_duration_ns = u64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6], payload[7],
                        ]);
                    }
                }
                0xE0 => {
                    video_payload = Some(payload);
                }
                0xE1 => {
                    audio_payload = Some(payload);
                }
                _ => {}
            }

            offset = payload_off + size;
        }

        if track_type == 1 {
            let mut v = VideoTrack::default();
            v.stream_id = stream_id;
            v.codec_id = Some(codec_id.clone());
            v.title = name;
            v.language = language;
            v.default_flag = default_flag;
            v.forced_flag = forced_flag;

            if default_duration_ns > 0 {
                v.frame_rate = Some(1_000_000_000.0 / default_duration_ns as f64);
            }

            if let Some(vp) = video_payload {
                Self::parse_video_settings(vp, &mut v);
            }

            if codec_id == "V_MPEG4/ISO/AVC" && !codec_private.is_empty() {
                v.format = VideoCodec::AVC;
                v.format_info = Some("Advanced Video Coding".to_string());
                if codec_private.len() >= 8 {
                    let sps_len = u16::from_be_bytes([codec_private[6], codec_private[7]]) as usize;
                    if codec_private.len() >= 8 + sps_len {
                        if let Ok(sps) = AvcSps::parse(&codec_private[8..8 + sps_len]) {
                            v.width = sps.width;
                            v.height = sps.height;
                            v.format_profile = Some(sps.profile_name.to_string());
                            v.format_level = Some(sps.level_name);
                            v.bit_depth = sps.bit_depth;
                            v.chroma_subsampling = Some(sps.chroma_subsampling);
                            v.color_range = sps.color_range;
                            v.color_primaries = sps.color_primaries;
                            v.transfer_characteristics = sps.transfer_characteristics;
                            v.matrix_coefficients = sps.matrix_coefficients;
                        }
                    }
                }
            } else if (codec_id == "V_MPEGH/ISO/HEVC" || codec_id == "V_MS/VFW/FOURCC") && !codec_private.is_empty() {
                v.format = VideoCodec::HEVC;
                v.format_info = Some("High Efficiency Video Coding".to_string());
                if codec_private.len() >= 23 {
                    let sps_bytes = &codec_private[23..];
                    if let Ok(sps) = HevcSps::parse(sps_bytes) {
                        v.width = sps.width;
                        v.height = sps.height;
                        v.format_profile = Some(sps.profile_name.to_string());
                        v.format_level = Some(sps.level_name);
                        v.format_tier = Some(sps.tier.to_string());
                        v.bit_depth = sps.bit_depth;
                        v.chroma_subsampling = Some(sps.chroma_subsampling);
                        v.color_range = sps.color_range;
                        v.color_primaries = sps.color_primaries;
                        v.transfer_characteristics = sps.transfer_characteristics;
                        v.matrix_coefficients = sps.matrix_coefficients;
                        v.hdr_format = sps.hdr_format;
                    }
                }
            } else if codec_id == "V_AV1" {
                v.format = VideoCodec::AV1;
                v.format_info = Some("AOMedia Video 1".to_string());
            } else if codec_id == "V_VP9" {
                v.format = VideoCodec::VP9;
                v.format_info = Some("Google VP9".to_string());
            }

            report.videos.push(v);
        } else if track_type == 2 {
            let mut a = AudioTrack::default();
            a.stream_id = stream_id;
            a.codec_id = Some(codec_id.clone());
            a.title = name;
            a.language = language;
            a.default_flag = default_flag;
            a.forced_flag = forced_flag;

            if let Some(ap) = audio_payload {
                Self::parse_audio_settings(ap, &mut a);
            }

            if codec_id == "A_AAC" {
                a.format = AudioCodec::AAC;
                a.format_info = Some("Advanced Audio Coding".to_string());
                if !codec_private.is_empty() {
                    if let Ok(aac) = AacInfo::parse_audio_specific_config(&codec_private) {
                        a.sampling_rate = aac.sampling_rate;
                        a.channels = aac.channels;
                        a.channel_layout = Some(aac.channel_layout);
                        a.format_profile = Some(aac.profile.to_string());
                    }
                }
            } else if codec_id == "A_AC3" {
                a.format = AudioCodec::AC3;
                a.format_info = Some("Dolby Digital".to_string());
            } else if codec_id == "A_EAC3" {
                a.format = AudioCodec::EAC3;
                a.format_info = Some("Dolby Digital Plus".to_string());
            } else if codec_id == "A_DTS" {
                a.format = AudioCodec::DTS;
                a.format_info = Some("DTS Digital Surround".to_string());
            } else if codec_id == "A_FLAC" {
                a.format = AudioCodec::FLAC;
                a.format_info = Some("Free Lossless Audio Codec".to_string());
                if !codec_private.is_empty() {
                    if let Ok(flac) = FlacStreamInfo::parse(&codec_private) {
                        a.sampling_rate = flac.sample_rate;
                        a.channels = flac.channels;
                        a.channel_layout = Some(flac.channel_layout);
                        a.bit_depth = Some(flac.bit_depth);
                        a.compression_mode = Some("Lossless".to_string());
                    }
                }
            } else if codec_id == "A_OPUS" {
                a.format = AudioCodec::Opus;
                a.format_info = Some("Opus Audio".to_string());
                if !codec_private.is_empty() {
                    if let Ok(opus) = OpusHead::parse(&codec_private) {
                        a.channels = opus.channels;
                        a.channel_layout = Some(opus.channel_layout);
                        a.sampling_rate = opus.output_sample_rate;
                    }
                }
            }

            report.audios.push(a);
        } else if track_type == 17 {
            let mut s = TextTrack::default();
            s.stream_id = stream_id;
            s.codec_id = Some(codec_id.clone());
            s.title = name;
            s.language = language;
            s.default_flag = default_flag;
            s.forced_flag = forced_flag;

            if codec_id == "S_TEXT/UTF8" {
                s.format = SubtitleCodec::SubRip;
                s.format_info = Some("SubRip (SRT)".to_string());
            } else if codec_id == "S_TEXT/ASS" {
                s.format = SubtitleCodec::ASS;
                s.format_info = Some("Advanced SubStation Alpha".to_string());
            } else if codec_id == "S_HDMV/PGS" {
                s.format = SubtitleCodec::PGS;
                s.format_info = Some("Presentation Graphic Stream".to_string());
            }

            report.texts.push(s);
        }
    }

    fn parse_video_settings(data: &[u8], track: &mut VideoTrack) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0xB0 => {
                    if size == 2 {
                        track.width = u16::from_be_bytes([payload[0], payload[1]]) as u32;
                    } else if size == 4 {
                        track.width = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    }
                }
                0xBA => {
                    if size == 2 {
                        track.height = u16::from_be_bytes([payload[0], payload[1]]) as u32;
                    } else if size == 4 {
                        track.height = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    }
                }
                0x55B0 => {
                    Self::parse_colour_element(payload, track);
                }
                _ => {}
            }

            offset = payload_off + size;
        }
    }

    fn parse_colour_element(data: &[u8], track: &mut VideoTrack) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0x55B1 => {
                    if let Some(&val) = payload.first() {
                        track.matrix_coefficients = Some(MatrixCoefficients::from_u8(val));
                    }
                }
                0x55B2 => {
                    if let Some(&val) = payload.first() {
                        track.bit_depth = val;
                    }
                }
                0x55B9 => {
                    if let Some(&val) = payload.first() {
                        track.color_range = Some(if val == 1 { ColorRange::Full } else { ColorRange::Limited });
                    }
                }
                0x55BA => {
                    if let Some(&val) = payload.first() {
                        track.transfer_characteristics = Some(TransferCharacteristics::from_u8(val));
                    }
                }
                0x55BB => {
                    if let Some(&val) = payload.first() {
                        track.color_primaries = Some(ColorPrimaries::from_u8(val));
                    }
                }
                0x55BC => {
                    let max_cll = if size == 2 {
                        u16::from_be_bytes([payload[0], payload[1]]) as u32
                    } else if size == 4 {
                        u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                    } else {
                        0
                    };
                    if track.content_light_level.is_none() {
                        track.content_light_level = Some(ContentLightLevel { max_cll, max_fall: 0 });
                    } else if let Some(ref mut cll) = track.content_light_level {
                        cll.max_cll = max_cll;
                    }
                }
                0x55BD => {
                    let max_fall = if size == 2 {
                        u16::from_be_bytes([payload[0], payload[1]]) as u32
                    } else if size == 4 {
                        u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                    } else {
                        0
                    };
                    if track.content_light_level.is_none() {
                        track.content_light_level = Some(ContentLightLevel { max_cll: 0, max_fall });
                    } else if let Some(ref mut cll) = track.content_light_level {
                        cll.max_fall = max_fall;
                    }
                }
                _ => {}
            }

            offset = payload_off + size;
        }
    }

    fn parse_audio_settings(data: &[u8], track: &mut AudioTrack) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            match id {
                0xB5 => {
                    if size == 4 {
                        track.sampling_rate = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as u32;
                    } else if size == 8 {
                        track.sampling_rate = f64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6], payload[7],
                        ]) as u32;
                    }
                }
                0x9F => {
                    track.channels = payload.first().copied().unwrap_or(2) as u32;
                    track.channel_layout = match track.channels {
                        1 => Some(AudioChannelLayout::Mono),
                        2 => Some(AudioChannelLayout::Stereo),
                        6 => Some(AudioChannelLayout::Surround5_1),
                        8 => Some(AudioChannelLayout::Surround7_1),
                        _ => Some(AudioChannelLayout::Stereo),
                    };
                }
                0x6264 => {
                    track.bit_depth = payload.first().copied();
                }
                _ => {}
            }

            offset = payload_off + size;
        }
    }

    fn parse_chapters(data: &[u8], report: &mut MediaReport) {
        let mut menu = MenuTrack::default();
        let mut offset = 0;

        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0x45B9 {
                Self::parse_edition_entry(payload, &mut menu);
            }

            offset = payload_off + size;
        }

        if !menu.chapters.is_empty() {
            report.menus.push(menu);
        }
    }

    fn parse_edition_entry(data: &[u8], menu: &mut MenuTrack) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0xB6 {
                let mut time_start_ns = 0u64;
                let mut title = String::new();

                let mut atom_off = 0;
                while atom_off < payload.len() {
                    let (aid, aid_len) = match EbmlVint::read_element_id(payload, atom_off) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let (asize_opt, asize_len) = match EbmlVint::read_element_size(payload, atom_off + aid_len) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let asize = asize_opt.unwrap_or(0) as usize;
                    let apayload_off = atom_off + aid_len + asize_len;
                    if apayload_off + asize > payload.len() {
                        break;
                    }
                    let apayload = &payload[apayload_off..apayload_off + asize];

                    if aid == 0x91 {
                        if asize == 4 {
                            time_start_ns = u32::from_be_bytes([apayload[0], apayload[1], apayload[2], apayload[3]]) as u64;
                        } else if asize == 8 {
                            time_start_ns = u64::from_be_bytes([
                                apayload[0], apayload[1], apayload[2], apayload[3], apayload[4], apayload[5], apayload[6], apayload[7],
                            ]);
                        }
                    } else if aid == 0x80 {
                        if apayload.windows(4).any(|w| w.starts_with(&[0x85])) {
                            title = String::from_utf8_lossy(&apayload[2..]).to_string();
                        }
                    }

                    atom_off = apayload_off + asize;
                }

                menu.chapters.push(Chapter {
                    timestamp_ms: (time_start_ns as f64) / 1_000_000.0,
                    title: if title.is_empty() { format!("Chapter {}", menu.chapters.len() + 1) } else { title },
                    language: None,
                });
            }

            offset = payload_off + size;
        }
    }

    fn parse_attachments(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0x61A7 {
                let mut name = String::new();
                let mut mime = String::new();
                let mut data_size = 0;

                let mut att_off = 0;
                while att_off < payload.len() {
                    let (aid, aid_len) = match EbmlVint::read_element_id(payload, att_off) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let (asize_opt, asize_len) = match EbmlVint::read_element_size(payload, att_off + aid_len) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let asize = asize_opt.unwrap_or(0) as usize;
                    let apayload_off = att_off + aid_len + asize_len;
                    if apayload_off + asize > payload.len() {
                        break;
                    }
                    let apayload = &payload[apayload_off..apayload_off + asize];

                    if aid == 0x467E {
                        name = String::from_utf8_lossy(apayload).to_string();
                    } else if aid == 0x4660 {
                        mime = String::from_utf8_lossy(apayload).to_string();
                    } else if aid == 0x465C {
                        data_size = asize;
                    }

                    att_off = apayload_off + asize;
                }

                if name.starts_with("cover") {
                    report.general.cover_art_present = true;
                    report.general.cover_mime = Some(mime.clone());
                }

                report.attachments.push(Attachment {
                    name,
                    mime_type: mime,
                    description: None,
                    size: data_size,
                    data_base64: None,
                });
            }

            offset = payload_off + size;
        }
    }

    fn parse_tags(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        while offset < data.len() {
            let (id, id_len) = match EbmlVint::read_element_id(data, offset) {
                Ok(res) => res,
                Err(_) => break,
            };
            let (size_opt, size_len) = match EbmlVint::read_element_size(data, offset + id_len) {
                Ok(res) => res,
                Err(_) => break,
            };
            let size = size_opt.unwrap_or(0) as usize;
            let payload_off = offset + id_len + size_len;
            if payload_off + size > data.len() {
                break;
            }
            let payload = &data[payload_off..payload_off + size];

            if id == 0x7373 {
                let tag_str = String::from_utf8_lossy(payload);
                if let Some((k, v)) = tag_str.split_once('=') {
                    report.general.tags.insert(k.to_string(), v.to_string());
                }
            }

            offset = payload_off + size;
        }
    }
}
