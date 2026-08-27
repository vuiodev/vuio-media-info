use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use std::collections::{HashMap, HashSet};

/// MPEG-TS (Transport Stream / M2TS) packet demuxer.
pub struct MpegTsDemuxer;

impl MpegTsDemuxer {
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

        let mut offset = 0;
        let mut pmt_pids = HashSet::new();
        let mut streams = HashMap::new(); // PID -> (stream_type, stream_kind)

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
                payload_offset += 1 + adapt_len;
            }

            if payload_offset < 188 {
                let payload = &pkt[payload_offset..];

                if pid == 0x0000 && payload_unit_start_indicator {
                    // PAT (Program Association Table)
                    let pointer_field = payload[0] as usize;
                    let table_off = 1 + pointer_field;
                    if table_off + 8 < payload.len() {
                        let section_len = (((payload[table_off + 1] & 0x0F) as usize) << 8) | (payload[table_off + 2] as usize);
                        let mut p_off = table_off + 8;
                        let section_end = (table_off + 3 + section_len).min(payload.len().saturating_sub(4));
                        while p_off + 4 <= section_end {
                            let prog_num = u16::from_be_bytes([payload[p_off], payload[p_off + 1]]);
                            let pmt_pid = (((payload[p_off + 2] & 0x1F) as u16) << 8) | (payload[p_off + 3] as u16);
                            if prog_num != 0 {
                                pmt_pids.insert(pmt_pid);
                            }
                            p_off += 4;
                        }
                    }
                } else if pmt_pids.contains(&pid) && payload_unit_start_indicator {
                    // PMT (Program Map Table)
                    let pointer_field = payload[0] as usize;
                    let table_off = 1 + pointer_field;
                    if table_off + 12 < payload.len() {
                        let section_len = (((payload[table_off + 1] & 0x0F) as usize) << 8) | (payload[table_off + 2] as usize);
                        let prog_info_len = (((payload[table_off + 10] & 0x0F) as usize) << 8) | (payload[table_off + 11] as usize);
                        let mut es_off = table_off + 12 + prog_info_len;
                        let section_end = (table_off + 3 + section_len).min(payload.len().saturating_sub(4));

                        while es_off + 5 <= section_end {
                            let stream_type = payload[es_off];
                            let elem_pid = (((payload[es_off + 1] & 0x1F) as u16) << 8) | (payload[es_off + 2] as u16);
                            let es_info_len = (((payload[es_off + 3] & 0x0F) as usize) << 8) | (payload[es_off + 4] as usize);

                            streams.insert(elem_pid, stream_type);
                            es_off += 5 + es_info_len;
                        }
                    }
                }
            }

            offset += packet_size;
            // Stop scanning PMT after 5000 packets for fast probing
            if offset > packet_size * 5000 && !streams.is_empty() {
                break;
            }
        }

        let mut stream_id = 1u32;
        for (pid, stream_type) in streams {
            match stream_type {
                0x1B => {
                    // H.264 / AVC
                    let mut v = VideoTrack::default();
                    v.stream_id = stream_id;
                    v.codec_id = Some(format!("PID {pid} (0x{pid:X})"));
                    v.format = VideoCodec::AVC;
                    v.format_info = Some("Advanced Video Coding".to_string());
                    report.videos.push(v);
                    stream_id += 1;
                }
                0x24 => {
                    // H.265 / HEVC
                    let mut v = VideoTrack::default();
                    v.stream_id = stream_id;
                    v.codec_id = Some(format!("PID {pid} (0x{pid:X})"));
                    v.format = VideoCodec::HEVC;
                    v.format_info = Some("High Efficiency Video Coding".to_string());
                    report.videos.push(v);
                    stream_id += 1;
                }
                0x02 => {
                    // MPEG-2 Video
                    let mut v = VideoTrack::default();
                    v.stream_id = stream_id;
                    v.codec_id = Some(format!("PID {pid} (0x{pid:X})"));
                    v.format = VideoCodec::MPEG2Video;
                    v.format_info = Some("MPEG Video".to_string());
                    report.videos.push(v);
                    stream_id += 1;
                }
                0x0F | 0x11 => {
                    // AAC Audio
                    let mut a = AudioTrack::default();
                    a.stream_id = stream_id;
                    a.codec_id = Some(format!("PID {pid} (0x{pid:X})"));
                    a.format = AudioCodec::AAC;
                    a.format_info = Some("Advanced Audio Coding".to_string());
                    report.audios.push(a);
                    stream_id += 1;
                }
                0x81 | 0x06 => {
                    // AC-3 / E-AC-3 / Teletext / PGS
                    let mut a = AudioTrack::default();
                    a.stream_id = stream_id;
                    a.codec_id = Some(format!("PID {pid} (0x{pid:X})"));
                    a.format = AudioCodec::AC3;
                    a.format_info = Some("Dolby Digital (AC-3)".to_string());
                    report.audios.push(a);
                    stream_id += 1;
                }
                _ => {}
            }
        }

        Ok(report)
    }
}
