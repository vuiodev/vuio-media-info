use crate::audio::{AacInfo, Ac3Header, DtsHeader, FlacStreamInfo, OpusHead, TrueHdHeader};
use crate::core::{bitstream::EbmlVint, error::Result, models::*, types::*};
use crate::video::{AvcSps, HevcSps};

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
            let element_size =
                size_opt.unwrap_or(data.len() as u64 - (offset + header_len) as u64) as usize;
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
            let payload = if payload_off + size <= data.len() {
                &data[payload_off..payload_off + size]
            } else {
                &data[payload_off..]
            };

            match id {
                0x1549A966 => {
                    // Info
                    Self::parse_info(payload, &mut timecode_scale, report);
                }
                0x1654AE6B => {
                    // Tracks
                    Self::parse_tracks(payload, timecode_scale, report);
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
                0x1F43B675 => {
                    // Cluster
                    Self::parse_cluster_blocks(payload, report);
                }
                0xA3 | 0xA0 => {
                    // Direct SimpleBlock or BlockGroup
                    Self::parse_cluster_blocks(&data[offset..], report);
                }
                _ => {}
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
                        duration_float =
                            f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                                as f64;
                    } else if size == 8 {
                        duration_float = f64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                            payload[6], payload[7],
                        ]);
                    }
                }
                0x7BA9 => {
                    // Title
                    report.general.title = Some(String::from_utf8_lossy(payload).to_string());
                }
                0x4D80 => {
                    // MuxingApp
                    report.general.encoded_application =
                        Some(String::from_utf8_lossy(payload).to_string());
                }
                0x5741 => {
                    // WritingApp
                    report.general.encoded_library =
                        Some(String::from_utf8_lossy(payload).to_string());
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

    fn parse_track_entry(
        data: &[u8],
        stream_id: u32,
        _timecode_scale: u64,
        report: &mut MediaReport,
    ) {
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

        let mut actual_track_number = stream_id;

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
                0xD7 => {
                    if size >= 1 && size <= 8 {
                        let mut val = 0u64;
                        for &b in payload.iter().take(size) {
                            val = (val << 8) | b as u64;
                        }
                        if val > 0 {
                            actual_track_number = val as u32;
                        }
                    }
                }
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
                        default_duration_ns =
                            u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                                as u64;
                    } else if size == 8 {
                        default_duration_ns = u64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                            payload[6], payload[7],
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
            v.stream_id = actual_track_number;
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
            } else if (codec_id == "V_MPEGH/ISO/HEVC" || codec_id == "V_MS/VFW/FOURCC")
                && !codec_private.is_empty()
            {
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
            a.stream_id = actual_track_number;
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
                if !codec_private.is_empty() {
                    if let Ok(ac3) = Ac3Header::parse(&codec_private) {
                        a.bit_rate = Some(ac3.bit_rate);
                        a.sampling_rate = ac3.sample_rate;
                        a.channels = ac3.channels;
                        a.channel_layout = Some(ac3.channel_layout);
                        a.bit_depth = Some(24);
                    }
                }
            } else if codec_id == "A_EAC3" {
                a.format = AudioCodec::EAC3;
                a.format_info = Some("Dolby Digital Plus".to_string());
                if !codec_private.is_empty() {
                    if let Ok(ac3) = Ac3Header::parse(&codec_private) {
                        a.bit_rate = Some(ac3.bit_rate);
                        a.sampling_rate = ac3.sample_rate;
                        a.channels = ac3.channels;
                        a.channel_layout = Some(ac3.channel_layout);
                        a.bit_depth = Some(24);
                        if ac3.dolby_atmos_present {
                            a.format_info =
                                Some("Dolby Digital Plus with Dolby Atmos (JOC)".to_string());
                        }
                    }
                }
            } else if codec_id.starts_with("A_DTS") {
                a.format = AudioCodec::DTS;
                a.format_info = Some("DTS Digital Surround".to_string());
                if !codec_private.is_empty() {
                    if let Ok(dts) = DtsHeader::parse(&codec_private) {
                        a.bit_rate = Some(dts.bit_rate);
                        a.sampling_rate = dts.sample_rate;
                        a.channels = dts.channels;
                        a.channel_layout = Some(dts.channel_layout);
                        a.format_profile = Some(dts.profile_name.to_string());
                    }
                }
            } else if codec_id == "A_TRUEHD" || codec_id == "A_MLP" {
                a.format = AudioCodec::TrueHD;
                a.format_info = Some("Dolby TrueHD".to_string());
                if !codec_private.is_empty() {
                    if let Ok(thd) = TrueHdHeader::parse(&codec_private) {
                        a.sampling_rate = thd.sample_rate;
                        a.channels = thd.channels;
                        a.channel_layout = Some(thd.channel_layout);
                        a.format_profile = Some(thd.format_profile);
                        a.bit_depth = Some(thd.bit_depth);
                    }
                }
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
            s.stream_id = actual_track_number;
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
                        track.width =
                            u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    }
                }
                0xBA => {
                    if size == 2 {
                        track.height = u16::from_be_bytes([payload[0], payload[1]]) as u32;
                    } else if size == 4 {
                        track.height =
                            u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
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
                        track.color_range = Some(if val == 1 {
                            ColorRange::Full
                        } else {
                            ColorRange::Limited
                        });
                    }
                }
                0x55BA => {
                    if let Some(&val) = payload.first() {
                        track.transfer_characteristics =
                            Some(TransferCharacteristics::from_u8(val));
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
                        track.content_light_level = Some(ContentLightLevel {
                            max_cll,
                            max_fall: 0,
                        });
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
                        track.content_light_level = Some(ContentLightLevel {
                            max_cll: 0,
                            max_fall,
                        });
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
                        track.sampling_rate =
                            f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                                as u32;
                    } else if size == 8 {
                        track.sampling_rate = f64::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                            payload[6], payload[7],
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
                    let (asize_opt, asize_len) =
                        match EbmlVint::read_element_size(payload, atom_off + aid_len) {
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
                            time_start_ns = u32::from_be_bytes([
                                apayload[0],
                                apayload[1],
                                apayload[2],
                                apayload[3],
                            ]) as u64;
                        } else if asize == 8 {
                            time_start_ns = u64::from_be_bytes([
                                apayload[0],
                                apayload[1],
                                apayload[2],
                                apayload[3],
                                apayload[4],
                                apayload[5],
                                apayload[6],
                                apayload[7],
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
                    title: if title.is_empty() {
                        format!("Chapter {}", menu.chapters.len() + 1)
                    } else {
                        title
                    },
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
                    let (asize_opt, asize_len) =
                        match EbmlVint::read_element_size(payload, att_off + aid_len) {
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

    fn parse_cluster_blocks(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        let mut probed_tracks: std::collections::HashSet<u32> = std::collections::HashSet::new();

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

            if id == 0xA3 || id == 0xA1 {
                // SimpleBlock or Block
                Self::probe_block_for_audio(payload, &mut probed_tracks, report);
            } else if id == 0xA0 {
                // BlockGroup
                let mut b_off = 0;
                while b_off < payload.len() {
                    let (bid, bid_len) = match EbmlVint::read_element_id(payload, b_off) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let (bsize_opt, bsize_len) =
                        match EbmlVint::read_element_size(payload, b_off + bid_len) {
                            Ok(res) => res,
                            Err(_) => break,
                        };
                    let bsize = bsize_opt.unwrap_or(0) as usize;
                    let bpay_off = b_off + bid_len + bsize_len;
                    if bpay_off + bsize > payload.len() {
                        break;
                    }
                    if bid == 0xA1 || bid == 0xA3 {
                        Self::probe_block_for_audio(
                            &payload[bpay_off..bpay_off + bsize],
                            &mut probed_tracks,
                            report,
                        );
                    }
                    b_off = bpay_off + bsize;
                }
            }

            let all_audios_have_bitrate =
                !report.audios.is_empty() && report.audios.iter().all(|a| a.bit_rate.is_some());
            if all_audios_have_bitrate {
                break;
            }

            offset = payload_off + size;
        }
    }

    fn probe_block_for_audio(
        block_data: &[u8],
        probed_tracks: &mut std::collections::HashSet<u32>,
        report: &mut MediaReport,
    ) {
        if block_data.len() < 4 {
            return;
        }

        // 1. Read TrackNumber VINT
        let (track_num_u64, vint_len) = match EbmlVint::read_element_size(block_data, 0) {
            Ok((Some(val), len)) => (val, len),
            _ => return,
        };
        let track_num = track_num_u64 as u32;

        if probed_tracks.contains(&track_num) {
            return;
        }

        // 2. Skip TrackNumber VINT (vint_len) + Timecode (2 bytes) + Flags (1 byte)
        if block_data.len() < vint_len + 3 {
            return;
        }

        let flags = block_data[vint_len + 2];
        let lacing = (flags >> 1) & 0x03;
        let mut frame_start = vint_len + 3;

        if lacing == 1 {
            // Xiph lacing
            if block_data.len() <= frame_start {
                return;
            }
            let frame_count = block_data[frame_start] as usize + 1;
            frame_start += 1;
            for _ in 0..frame_count - 1 {
                while frame_start < block_data.len() && block_data[frame_start] == 255 {
                    frame_start += 1;
                }
                frame_start += 1;
            }
        } else if lacing == 3 {
            // Fixed-size lacing
            frame_start += 1;
        } else if lacing == 2 {
            // EBML lacing
            if block_data.len() <= frame_start {
                return;
            }
            let _frame_count = block_data[frame_start] as usize + 1;
            frame_start += 1;
            if let Ok((_, elen)) = EbmlVint::read_element_size(block_data, frame_start) {
                frame_start += elen;
            }
        }

        if frame_start >= block_data.len() {
            return;
        }

        let frame_payload = &block_data[frame_start..];

        // Find target audio track
        if let Some(audio) = report.audios.iter_mut().find(|a| a.stream_id == track_num) {
            if audio.bit_rate.is_some() {
                probed_tracks.insert(track_num);
                return;
            }

            // Check for AC-3 / E-AC-3 (0x0B77)
            if let Some(pos) = frame_payload
                .windows(2)
                .position(|w| w == [0x0B, 0x77] || w == [0x77, 0x0B])
            {
                let slice = if frame_payload[pos] == 0x77 {
                    let mut swapped = Vec::with_capacity(frame_payload.len() - pos);
                    for chunk in frame_payload[pos..].as_chunks::<2>().0 {
                        swapped.push(chunk[1]);
                        swapped.push(chunk[0]);
                    }
                    swapped
                } else {
                    frame_payload[pos..].to_vec()
                };

                if let Ok(ac3) = Ac3Header::parse(&slice) {
                    audio.bit_rate = Some(ac3.bit_rate);
                    audio.sampling_rate = ac3.sample_rate;
                    audio.channels = ac3.channels;
                    audio.channel_layout = Some(ac3.channel_layout);
                    audio.bit_depth = Some(24);
                    if ac3.is_eac3 {
                        audio.format = AudioCodec::EAC3;
                        if ac3.dolby_atmos_present {
                            audio.format_info =
                                Some("Dolby Digital Plus with Dolby Atmos (JOC)".to_string());
                        } else {
                            audio.format_info = Some("Dolby Digital Plus".to_string());
                        }
                    } else {
                        audio.format = AudioCodec::AC3;
                        audio.format_info = Some("Dolby Digital".to_string());
                        audio.format_profile = Some(if ac3.channels == 6 {
                            "Dolby Digital 5.1".to_string()
                        } else {
                            "Dolby Digital Stereo".to_string()
                        });
                    }
                    probed_tracks.insert(track_num);
                    return;
                }
            }

            // Check for DTS (0x7FFE8001 / 0x1FFFE800 / 0xFE7F0180)
            if let Some(pos) = frame_payload.windows(4).position(|w| {
                w == [0x7F, 0xFE, 0x80, 0x01]
                    || w == [0x1F, 0xFF, 0xE8, 0x00]
                    || w == [0xFE, 0x7F, 0x01, 0x80]
            }) {
                if let Ok(dts) = DtsHeader::parse(&frame_payload[pos..]) {
                    audio.bit_rate = Some(dts.bit_rate);
                    audio.sampling_rate = dts.sample_rate;
                    audio.channels = dts.channels;
                    audio.channel_layout = Some(dts.channel_layout);
                    audio.format_profile = Some(dts.profile_name.to_string());
                    probed_tracks.insert(track_num);
                    return;
                }
            }

            // Check for TrueHD / MLP (0xF8726FBA / 0xF8726FA9)
            if let Some(pos) = frame_payload
                .windows(4)
                .position(|w| w == [0xF8, 0x72, 0x6F, 0xBA] || w == [0xF8, 0x72, 0x6F, 0xA9])
            {
                if let Ok(thd) = TrueHdHeader::parse(&frame_payload[pos..]) {
                    audio.sampling_rate = thd.sample_rate;
                    audio.channels = thd.channels;
                    audio.channel_layout = Some(thd.channel_layout);
                    audio.format_profile = Some(thd.format_profile);
                    audio.bit_depth = Some(thd.bit_depth);
                    probed_tracks.insert(track_num);
                }
            }
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
                // Tag element
                Self::parse_tag_entry(payload, report);
            }

            offset = payload_off + size;
        }
    }

    fn parse_tag_entry(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        let mut name = String::new();
        let mut value = String::new();

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

            if id == 0x67C8 {
                // SimpleTag -> TagName (0x45A3) + TagString (0x4487)
                let mut tag_off = 0;
                while tag_off < payload.len() {
                    let (tid, tid_len) = match EbmlVint::read_element_id(payload, tag_off) {
                        Ok(res) => res,
                        Err(_) => break,
                    };
                    let (tsize_opt, tsize_len) =
                        match EbmlVint::read_element_size(payload, tag_off + tid_len) {
                            Ok(res) => res,
                            Err(_) => break,
                        };
                    let tsize = tsize_opt.unwrap_or(0) as usize;
                    let tpay_off = tag_off + tid_len + tsize_len;
                    if tpay_off + tsize > payload.len() {
                        break;
                    }
                    let tpay = &payload[tpay_off..tpay_off + tsize];

                    if tid == 0x45A3 {
                        name = String::from_utf8_lossy(tpay).to_string();
                    } else if tid == 0x4487 {
                        value = String::from_utf8_lossy(tpay).to_string();
                    }

                    tag_off = tpay_off + tsize;
                }
            }

            offset = payload_off + size;
        }

        if !name.is_empty() && !value.is_empty() {
            if name.starts_with("BPS") {
                if let Ok(bps_val) = value.parse::<u64>() {
                    for a in &mut report.audios {
                        if a.bit_rate.is_none() {
                            a.bit_rate = Some(bps_val);
                            break;
                        }
                    }
                }
            }
            report.general.tags.insert(name, value);
        }
    }
}
