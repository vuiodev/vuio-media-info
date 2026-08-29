use crate::audio::{AacInfo, Ac3Header, DtsHeader, MpegaHeader};
use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use crate::video::{AvcSps, HevcSps, Mpeg2SequenceHeader};
use std::collections::{HashMap, HashSet};

/// MPEG-TS (Transport Stream / M2TS) packet demuxer.
pub struct MpegTsDemuxer;

impl MpegTsDemuxer {
    /// Removes the PES header so the caller sees only elementary stream bytes.
    fn strip_pes_header(payload: &[u8]) -> Option<&[u8]> {
        if payload.len() < 9 || payload[0] != 0 || payload[1] != 0 || payload[2] != 1 {
            return Some(payload);
        }
        let header_len = payload[8] as usize;
        payload.get(9 + header_len..)
    }

    /// Finds the first SPS NAL unit in an Annex B stream and applies it.
    fn apply_avc(data: &[u8], v: &mut VideoTrack) {
        let Some(sps) = Self::find_nal(data, 0x1F, 7) else {
            return;
        };
        let Ok(sps) = AvcSps::parse(sps) else {
            return;
        };
        v.width = sps.width;
        v.height = sps.height;
        v.format_profile = Some(sps.profile_name.to_string());
        v.format_level = Some(sps.level_name);
        v.bit_depth = sps.bit_depth;
        v.chroma_subsampling = Some(sps.chroma_subsampling);
        v.color_range = sps.color_range.or(v.color_range);
        v.color_primaries = sps.color_primaries;
        v.transfer_characteristics = sps.transfer_characteristics;
        v.matrix_coefficients = sps.matrix_coefficients;
        v.frame_rate = sps.frame_rate;
        v.scan_type = Some(
            if sps.progressive {
                "Progressive"
            } else {
                "Interlaced"
            }
            .to_string(),
        );
    }

    fn apply_hevc(data: &[u8], v: &mut VideoTrack) {
        // HEVC NAL type is bits 6..1 of the first header byte; SPS is type 33.
        let Some(sps) = Self::find_nal(data, 0x7E, 33 << 1) else {
            return;
        };
        let Ok(sps) = HevcSps::parse(sps) else {
            return;
        };
        v.width = sps.width;
        v.height = sps.height;
        v.format_profile = Some(sps.profile_name.to_string());
        v.format_level = Some(sps.level_name);
        v.format_tier = Some(sps.tier.to_string());
        v.bit_depth = sps.bit_depth;
        v.chroma_subsampling = Some(sps.chroma_subsampling);
        v.color_range = sps.color_range.or(v.color_range);
        v.color_primaries = sps.color_primaries;
        v.transfer_characteristics = sps.transfer_characteristics;
        v.matrix_coefficients = sps.matrix_coefficients;
        v.hdr_format = sps.hdr_format;
        v.frame_rate = sps.frame_rate;
    }

    /// Scans Annex B start codes for a NAL whose masked header byte matches `want`.
    fn find_nal(data: &[u8], mask: u8, want: u8) -> Option<&[u8]> {
        let mut i = 0;
        while i + 4 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                if data[i + 3] & mask == want {
                    return Some(&data[i + 3..]);
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        None
    }

    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 188 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 188,
                actual: data.len(),
            });
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MPEGTS;
        report.general.file_size = data.len() as u64;

        // Detect packet size: 188 (standard TS) or 192 (M2TS / Blu-ray with timestamp prefix)
        let packet_size = if data.len() >= 192 * 3 && data[4] == 0x47 && data[196] == 0x47 {
            192
        } else {
            188
        };
        let packet_prefix = if packet_size == 192 { 4 } else { 0 };
        if packet_size == 192 {
            // The 4-byte arrival timestamp prefix identifies a Blu-ray BDAV stream.
            report.general.format = ContainerFormat::BDAV;
        }

        let mut offset = 0;
        let mut pmt_pids = HashSet::new();
        let mut streams = HashMap::new(); // PID -> stream_type
        // Leading bytes of each elementary stream, for bitstream-level probing.
        let mut es_head: HashMap<u16, Vec<u8>> = HashMap::new();
        // Elementary stream bytes per PID, with transport and PES framing removed.
        let mut es_bytes: HashMap<u16, u64> = HashMap::new();
        // Access units per PID, counted by payload-unit-start indicators.
        let mut es_units: HashMap<u16, u64> = HashMap::new();
        let mut first_pcr: Option<u64> = None;
        let mut last_pcr: Option<u64> = None;

        while offset + packet_size <= data.len() {
            let pkt = &data[offset + packet_prefix..offset + packet_size];
            if pkt[0] != 0x47 {
                offset += 1;
                continue;
            }

            let payload_unit_start_indicator = (pkt[1] & 0x40) != 0;
            let pid = (((pkt[1] & 0x1F) as u16) << 8) | (pkt[2] as u16);
            let adaptation_control = (pkt[3] >> 4) & 0x03;

            let mut payload_offset = 4;
            if adaptation_control == 2 || adaptation_control == 3 {
                let adapt_len = pkt[4] as usize;
                // The adaptation field may carry a Program Clock Reference.
                if adapt_len >= 7 && pkt.len() > 11 && (pkt[5] & 0x10) != 0 {
                    let base = ((pkt[6] as u64) << 25)
                        | ((pkt[7] as u64) << 17)
                        | ((pkt[8] as u64) << 9)
                        | ((pkt[9] as u64) << 1)
                        | ((pkt[10] as u64) >> 7);
                    if first_pcr.is_none() {
                        first_pcr = Some(base);
                    }
                    last_pcr = Some(base);
                }
                payload_offset += 1 + adapt_len;
            }

            if payload_offset < 188 {
                let payload = &pkt[payload_offset..];

                if pid == 0x0000 && payload_unit_start_indicator {
                    // PAT (Program Association Table)
                    let pointer_field = payload[0] as usize;
                    let table_off = 1 + pointer_field;
                    if table_off + 8 < payload.len() {
                        let section_len = (((payload[table_off + 1] & 0x0F) as usize) << 8)
                            | (payload[table_off + 2] as usize);
                        let mut p_off = table_off + 8;
                        let section_end =
                            (table_off + 3 + section_len).min(payload.len().saturating_sub(4));
                        while p_off + 4 <= section_end {
                            let prog_num = u16::from_be_bytes([payload[p_off], payload[p_off + 1]]);
                            let pmt_pid = (((payload[p_off + 2] & 0x1F) as u16) << 8)
                                | (payload[p_off + 3] as u16);
                            if prog_num != 0 {
                                pmt_pids.insert(pmt_pid);
                            }
                            p_off += 4;
                        }
                    }
                } else if streams.contains_key(&pid) {
                    // Elementary stream packet: strip the PES header on the first packet
                    // of each access unit before probing the bitstream.
                    let body = if payload_unit_start_indicator {
                        *es_units.entry(pid).or_insert(0) += 1;
                        Self::strip_pes_header(payload)
                    } else {
                        Some(payload)
                    };
                    if let Some(body) = body {
                        *es_bytes.entry(pid).or_insert(0) += body.len() as u64;
                        let head = es_head.entry(pid).or_default();
                        if head.len() < 65536 {
                            let want = 65536 - head.len();
                            head.extend_from_slice(&body[..body.len().min(want)]);
                        }
                    }
                } else if pmt_pids.contains(&pid) && payload_unit_start_indicator {
                    // PMT (Program Map Table)
                    let pointer_field = payload[0] as usize;
                    let table_off = 1 + pointer_field;
                    if table_off + 12 < payload.len() {
                        let section_len = (((payload[table_off + 1] & 0x0F) as usize) << 8)
                            | (payload[table_off + 2] as usize);
                        let prog_info_len = (((payload[table_off + 10] & 0x0F) as usize) << 8)
                            | (payload[table_off + 11] as usize);
                        let mut es_off = table_off + 12 + prog_info_len;
                        let section_end =
                            (table_off + 3 + section_len).min(payload.len().saturating_sub(4));

                        while es_off + 5 <= section_end {
                            let stream_type = payload[es_off];
                            let elem_pid = (((payload[es_off + 1] & 0x1F) as u16) << 8)
                                | (payload[es_off + 2] as u16);
                            let es_info_len = (((payload[es_off + 3] & 0x0F) as usize) << 8)
                                | (payload[es_off + 4] as usize);

                            streams.insert(elem_pid, stream_type);
                            es_off += 5 + es_info_len;
                        }
                    }
                }
            }

            offset += packet_size;
        }

        // PCR ticks at 90 kHz; its span across the file is the program duration.
        let duration_ms = match (first_pcr, last_pcr) {
            (Some(a), Some(b)) if b > a => Some((b - a) as f64 / 90.0),
            _ => None,
        };
        if let Some(ms) = duration_ms {
            report.general.duration_ms = Some(ms);
            report.general.overall_bitrate = Some(((data.len() * 8) as f64 / (ms / 1000.0)) as u64);
        }

        let mut pids: Vec<u16> = streams.keys().copied().collect();
        pids.sort_unstable();

        let mut stream_id = 1u32;
        for pid in pids {
            let Some(&stream_type) = streams.get(&pid) else {
                continue;
            };
            let head = es_head.get(&pid).map(Vec::as_slice).unwrap_or(&[]);
            let bytes = es_bytes.get(&pid).copied().unwrap_or(0);
            let units = es_units.get(&pid).copied().unwrap_or(0);

            match stream_type {
                // Video elementary streams
                0x01 | 0x02 | 0x10 | 0x1B | 0x24 | 0xEA => {
                    let mut v = VideoTrack::default();
                    v.stream_id = stream_id;
                    v.codec_id = Some(stream_type.to_string());
                    v.stream_size = Some(bytes);
                    v.frame_count = (units > 0).then_some(units);

                    match stream_type {
                        0x1B => {
                            v.format = VideoCodec::AVC;
                            v.format_info = Some("Advanced Video Coding".to_string());
                            Self::apply_avc(head, &mut v);
                        }
                        0x24 => {
                            v.format = VideoCodec::HEVC;
                            v.format_info = Some("High Efficiency Video Coding".to_string());
                            Self::apply_hevc(head, &mut v);
                        }
                        0x10 => {
                            v.format = VideoCodec::MPEG4Visual;
                            v.format_info = Some("MPEG-4 Visual".to_string());
                        }
                        0xEA => {
                            v.format = VideoCodec::VC1;
                            v.format_info = Some("SMPTE 421M".to_string());
                        }
                        _ => {
                            v.format = VideoCodec::MPEG2Video;
                            v.format_info = Some("MPEG Video".to_string());
                            if let Ok(seq) = Mpeg2SequenceHeader::parse(head) {
                                v.width = seq.width;
                                v.height = seq.height;
                                v.frame_rate = Some(seq.frame_rate);
                                v.chroma_subsampling = Some(seq.chroma_subsampling);
                                v.format_profile = seq.profile.clone();
                                v.format_level = seq.level.clone();
                            }
                        }
                    }

                    // A frame count and rate give a more accurate track length than the
                    // program clock, which spans muxing preroll as well as the frames.
                    v.duration_ms = match (v.frame_count, v.frame_rate) {
                        (Some(frames), Some(fps)) if fps > 0.0 => {
                            Some(frames as f64 / fps * 1000.0)
                        }
                        _ => duration_ms,
                    };
                    if let Some(ms) = v.duration_ms.filter(|ms| *ms > 0.0) {
                        v.bit_rate = Some((bytes as f64 * 8.0 / (ms / 1000.0)) as u64);
                    }
                    if v.width > 0 && v.height > 0 {
                        v.display_aspect_ratio = Some(v.width as f64 / v.height as f64);
                    }
                    report.videos.push(v);
                    stream_id += 1;
                }
                // Audio elementary streams
                0x03 | 0x04 | 0x0F | 0x11 | 0x81 | 0x87 | 0x82 | 0x85 | 0x86 => {
                    let mut a = AudioTrack::default();
                    a.stream_id = stream_id;
                    a.codec_id = Some(stream_type.to_string());
                    a.duration_ms = duration_ms;
                    a.stream_size = Some(bytes);
                    a.bit_rate = duration_ms
                        .filter(|ms| *ms > 0.0)
                        .map(|ms| (bytes as f64 * 8.0 / (ms / 1000.0)) as u64);

                    match stream_type {
                        0x0F | 0x11 => {
                            a.format = AudioCodec::AAC;
                            a.format_info = Some("Advanced Audio Coding".to_string());
                            if let Ok(aac) = AacInfo::parse_adts(head) {
                                a.sampling_rate = aac.sampling_rate;
                                a.channels = aac.channels;
                                a.channel_layout = Some(aac.channel_layout);
                                a.format_profile = Some(aac.profile.to_string());
                                a.codec_id =
                                    Some(format!("{stream_type}-{}", aac.audio_object_type));
                            }
                        }
                        0x03 | 0x04 => {
                            a.format = AudioCodec::MPEGAudioLayer2;
                            a.format_info = Some("MPEG Audio".to_string());
                            if let Ok(mpa) = MpegaHeader::parse(head) {
                                a.format = match mpa.layer {
                                    "Layer 1" => AudioCodec::MPEGAudioLayer1,
                                    "Layer 2" => AudioCodec::MPEGAudioLayer2,
                                    _ => AudioCodec::MPEGAudioLayer3,
                                };
                                a.format_profile = Some(mpa.layer.to_string());
                                a.sampling_rate = mpa.sample_rate;
                                a.channels = mpa.channels;
                                a.channel_layout = Some(mpa.channel_layout);
                                a.bit_rate = Some(mpa.bit_rate);
                            }
                        }
                        0x82 | 0x85 | 0x86 => {
                            a.format = AudioCodec::DTS;
                            a.format_info = Some("DTS".to_string());
                            if let Ok(dts) = DtsHeader::parse(head) {
                                a.sampling_rate = dts.sample_rate;
                                a.channels = dts.channels;
                                a.channel_layout = Some(dts.channel_layout);
                                a.bit_rate = Some(dts.bit_rate);
                                a.format_profile = Some(dts.profile_name.to_string());
                            }
                        }
                        _ => {
                            a.format = AudioCodec::AC3;
                            a.format_info = Some("Dolby Digital".to_string());
                            if let Ok(ac3) = Ac3Header::parse(head) {
                                a.format = if ac3.is_eac3 {
                                    AudioCodec::EAC3
                                } else {
                                    AudioCodec::AC3
                                };
                                a.sampling_rate = ac3.sample_rate;
                                a.channels = ac3.channels;
                                a.channel_layout = Some(ac3.channel_layout);
                                a.bit_rate = Some(ac3.bit_rate);
                                a.dialnorm_db = Some(ac3.dialnorm_db);
                            }
                        }
                    }
                    report.audios.push(a);
                    stream_id += 1;
                }
                // Subtitle elementary streams
                0x90 | 0x92 => {
                    let mut t = TextTrack::default();
                    t.stream_id = stream_id;
                    t.codec_id = Some(stream_type.to_string());
                    t.format = SubtitleCodec::PGS;
                    t.format_info = Some("Presentation Graphic Stream".to_string());
                    report.texts.push(t);
                    stream_id += 1;
                }
                _ => {}
            }
        }

        Ok(report)
    }
}
