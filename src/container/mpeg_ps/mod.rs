use crate::audio::{Ac3Header, DtsHeader, MpegaHeader};
use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use crate::video::{AvcSps, Mpeg2SequenceHeader};
use std::collections::HashMap;

const PACK_START: [u8; 4] = [0x00, 0x00, 0x01, 0xBA];

/// MPEG-1 / MPEG-2 Program Stream demuxer, covering `.mpg`, `.mpeg`, `.vob` and `.evob`.
pub struct MpegPsDemuxer;

/// Accumulated state for one elementary stream within the program stream.
#[derive(Default)]
struct PsStream {
    bytes: u64,
    /// The first payload bytes, kept for elementary stream header probing.
    head: Vec<u8>,
}

impl PsStream {
    fn push(&mut self, payload: &[u8]) {
        self.bytes += payload.len() as u64;
        if self.head.len() < 8192 {
            let want = 8192 - self.head.len();
            self.head
                .extend_from_slice(&payload[..payload.len().min(want)]);
        }
    }
}

impl MpegPsDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 4 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 4,
                actual: data.len(),
            });
        }
        if !data.starts_with(&PACK_START) {
            return Err(MediaInfoError::InvalidData(
                "Not a valid MPEG Program Stream (missing pack start code)".to_string(),
            ));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MPEGPS;
        report.general.file_size = data.len() as u64;

        // Key is the PES stream id, or 0x100 | substream id for private stream 1.
        let mut streams: HashMap<u16, PsStream> = HashMap::new();
        let mut first_scr = None;
        let mut last_scr = None;
        let mut first_pts = None;
        let mut last_pts = None;
        let mut mux_rate = 0u64;

        let mut offset = 0usize;
        while offset + 4 <= data.len() {
            if data[offset] != 0 || data[offset + 1] != 0 || data[offset + 2] != 1 {
                offset += 1;
                continue;
            }
            let start_code = data[offset + 3];

            match start_code {
                0xBA => {
                    let Some((scr, rate, len)) = Self::parse_pack_header(&data[offset..]) else {
                        break;
                    };
                    if first_scr.is_none() {
                        first_scr = Some(scr);
                    }
                    last_scr = Some(scr);
                    if rate > 0 {
                        mux_rate = rate;
                    }
                    offset += len;
                }
                0xB9 => break, // program end
                0xBB | 0xBE | 0xBF => {
                    // System header, padding and private stream 2 carry no elementary data.
                    let Some(len) = Self::pes_packet_len(&data[offset..]) else {
                        break;
                    };
                    offset += 6 + len;
                }
                0xC0..=0xEF | 0xBD | 0xFD => {
                    let Some(len) = Self::pes_packet_len(&data[offset..]) else {
                        break;
                    };
                    let packet_end = (offset + 6 + len).min(data.len());
                    let packet = &data[offset..packet_end];
                    if let Some((key, payload, pts)) = Self::pes_payload(packet, start_code) {
                        streams.entry(key).or_default().push(payload);
                        if let Some(pts) = pts {
                            first_pts = Some(first_pts.map_or(pts, |p: u64| p.min(pts)));
                            last_pts = Some(last_pts.map_or(pts, |p: u64| p.max(pts)));
                        }
                    }
                    offset = packet_end;
                    if len == 0 {
                        offset += 1;
                    }
                }
                _ => offset += 4,
            }
        }

        // Presentation timestamps bound the real playback time; the SCR span is only a
        // fallback because muxer preload makes it run past the last frame.
        let duration_ms = match (first_pts, last_pts) {
            (Some(a), Some(b)) if b > a => Some((b - a) as f64 / 90.0),
            _ => match (first_scr, last_scr) {
                (Some(a), Some(b)) if b > a => Some((b - a) as f64 / 90.0),
                _ => None,
            },
        };
        if let Some(ms) = duration_ms {
            report.general.duration_ms = Some(ms);
            report.general.overall_bitrate = Some(((data.len() * 8) as f64 / (ms / 1000.0)) as u64);
        } else if mux_rate > 0 {
            report.general.overall_bitrate = Some(mux_rate * 400);
        }

        let mut ids: Vec<u16> = streams.keys().copied().collect();
        let _ = mux_rate;
        ids.sort_unstable();
        let mut stream_id = 1u32;
        for key in ids {
            let Some(stream) = streams.get(&key) else {
                continue;
            };
            Self::build_track(key, stream, stream_id, duration_ms, &mut report);
            stream_id += 1;
        }

        if report.videos.is_empty() && report.audios.is_empty() && report.texts.is_empty() {
            return Err(MediaInfoError::InvalidData(
                "No elementary streams found in program stream".to_string(),
            ));
        }

        Ok(report)
    }

    /// Returns the SCR (90 kHz), the program mux rate and the pack header length.
    fn parse_pack_header(data: &[u8]) -> Option<(u64, u64, usize)> {
        let b = data.get(4..14)?;
        if (b[0] & 0xC0) == 0x40 {
            // MPEG-2: 33-bit SCR base plus a 9-bit extension, then a 22-bit mux rate.
            let scr = (((b[0] as u64 >> 3) & 0x07) << 30)
                | (((b[0] as u64) & 0x03) << 28)
                | ((b[1] as u64) << 20)
                | (((b[2] as u64 >> 3) & 0x1F) << 15)
                | (((b[2] as u64) & 0x03) << 13)
                | ((b[3] as u64) << 5)
                | ((b[4] as u64 >> 3) & 0x1F);
            let rate = ((b[6] as u64) << 14) | ((b[7] as u64) << 6) | ((b[8] as u64) >> 2);
            let stuffing = (b[9] & 0x07) as usize;
            Some((scr, rate, 14 + stuffing))
        } else if (b[0] & 0xF0) == 0x20 {
            // MPEG-1: 33-bit SCR and a 22-bit mux rate, no stuffing field.
            let scr = (((b[0] as u64 >> 1) & 0x07) << 30)
                | ((b[1] as u64) << 22)
                | (((b[2] as u64 >> 1) & 0x7F) << 15)
                | ((b[3] as u64) << 7)
                | ((b[4] as u64 >> 1) & 0x7F);
            let rate = (((b[5] as u64) & 0x7F) << 15) | ((b[6] as u64) << 7) | ((b[7] as u64) >> 1);
            Some((scr, rate, 12))
        } else {
            None
        }
    }

    fn pes_packet_len(data: &[u8]) -> Option<usize> {
        let hi = *data.get(4)? as usize;
        let lo = *data.get(5)? as usize;
        Some((hi << 8) | lo)
    }

    /// Decodes a 33-bit timestamp from its five marker-interleaved bytes.
    fn read_timestamp(b: &[u8]) -> Option<u64> {
        let b = b.get(..5)?;
        Some(
            (((b[0] as u64) >> 1) & 0x07) << 30
                | (b[1] as u64) << 22
                | (((b[2] as u64) >> 1) & 0x7F) << 15
                | (b[3] as u64) << 7
                | ((b[4] as u64) >> 1) & 0x7F,
        )
    }

    /// Strips the PES header and returns the stream key, its payload and any PTS.
    fn pes_payload(packet: &[u8], stream_id: u8) -> Option<(u16, &[u8], Option<u64>)> {
        let mut pos = 6usize;
        let mut pts = None;
        if (*packet.get(pos)? & 0xC0) == 0x80 {
            // MPEG-2 PES: flags, flags, header length.
            let pts_dts_flags = (*packet.get(pos + 1)? >> 6) & 0x03;
            let header_len = *packet.get(pos + 2)? as usize;
            if pts_dts_flags & 0x02 != 0 {
                pts = packet.get(pos + 3..).and_then(Self::read_timestamp);
            }
            pos += 3 + header_len;
        } else {
            // MPEG-1 PES: up to 16 stuffing bytes, an optional STD buffer, then timestamps.
            let mut stuffing = 0;
            while packet.get(pos) == Some(&0xFF) && stuffing < 16 {
                pos += 1;
                stuffing += 1;
            }
            if (*packet.get(pos)? & 0xC0) == 0x40 {
                pos += 2;
            }
            match *packet.get(pos)? & 0xF0 {
                0x20 => {
                    pts = packet.get(pos..).and_then(Self::read_timestamp);
                    pos += 5;
                }
                0x30 => {
                    pts = packet.get(pos..).and_then(Self::read_timestamp);
                    pos += 10;
                }
                _ => pos += 1,
            }
        }

        let payload = packet.get(pos..)?;
        if payload.is_empty() {
            return None;
        }

        if stream_id == 0xBD {
            // Private stream 1 multiplexes several substreams behind a substream id.
            let sub_id = payload[0];
            // AC-3/DTS carry a 3-byte frame header after the id; LPCM carries 6.
            let skip = match sub_id {
                0x80..=0x8F => 4,
                0xA0..=0xAF => 7,
                _ => 1,
            };
            let body = payload.get(skip..)?;
            return Some((0x100 | sub_id as u16, body, pts));
        }

        Some((stream_id as u16, payload, pts))
    }

    fn build_track(
        key: u16,
        stream: &PsStream,
        stream_id: u32,
        duration_ms: Option<f64>,
        report: &mut MediaReport,
    ) {
        let bit_rate = duration_ms
            .filter(|ms| *ms > 0.0)
            .map(|ms| (stream.bytes as f64 * 8.0 / (ms / 1000.0)) as u64);

        match key {
            // Video streams
            0xE0..=0xEF => {
                let mut v = VideoTrack::default();
                v.stream_id = stream_id;
                v.codec_id = Some(format!("{key:02X}"));
                v.duration_ms = duration_ms;
                v.stream_size = Some(stream.bytes);
                v.bit_rate = bit_rate;

                if let Ok(seq) = Mpeg2SequenceHeader::parse(&stream.head) {
                    v.format = if seq.is_mpeg2 {
                        VideoCodec::MPEG2Video
                    } else {
                        VideoCodec::MPEG1Video
                    };
                    v.format_info = Some(
                        if seq.is_mpeg2 {
                            "MPEG-2 Video"
                        } else {
                            "MPEG-1 Video"
                        }
                        .to_string(),
                    );
                    v.width = seq.width;
                    v.height = seq.height;
                    v.frame_rate = Some(seq.frame_rate);
                    v.frame_rate_mode = Some(FrameRateMode::Constant);
                    v.chroma_subsampling = Some(seq.chroma_subsampling);
                    v.format_profile = seq.profile.clone();
                    v.format_level = seq.level.clone();
                    if seq.bit_rate > 0 && v.bit_rate.is_none() {
                        v.bit_rate = Some(seq.bit_rate);
                    }
                } else if let Some(sps) = Self::find_avc_sps(&stream.head) {
                    v.format = VideoCodec::AVC;
                    v.format_info = Some("Advanced Video Coding".to_string());
                    v.width = sps.width;
                    v.height = sps.height;
                    v.format_profile = Some(sps.profile_name.to_string());
                    v.format_level = Some(sps.level_name);
                    v.bit_depth = sps.bit_depth;
                    v.chroma_subsampling = Some(sps.chroma_subsampling);
                    v.frame_rate = sps.frame_rate;
                } else {
                    v.format = VideoCodec::MPEG2Video;
                    v.format_info = Some("MPEG Video".to_string());
                }

                v.color_space = Some("YUV".to_string());
                if v.width > 0 && v.height > 0 {
                    v.display_aspect_ratio = Some(v.width as f64 / v.height as f64);
                }
                report.videos.push(v);
            }
            // MPEG audio streams
            0xC0..=0xDF => {
                let mut a = AudioTrack::default();
                a.stream_id = stream_id;
                a.codec_id = Some(format!("{key:02X}"));
                a.duration_ms = duration_ms;
                a.stream_size = Some(stream.bytes);
                a.bit_rate = bit_rate;
                a.format = AudioCodec::MPEGAudioLayer2;
                a.format_info = Some("MPEG Audio".to_string());
                if let Ok(mpa) = MpegaHeader::parse(&stream.head) {
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
                report.audios.push(a);
            }
            // Private stream 1: AC-3
            0x180..=0x187 => {
                let mut a = AudioTrack::default();
                a.stream_id = stream_id;
                a.codec_id = Some(format!("{:02X}", key & 0xFF));
                a.duration_ms = duration_ms;
                a.stream_size = Some(stream.bytes);
                a.bit_rate = bit_rate;
                a.format = AudioCodec::AC3;
                a.format_info = Some("Dolby Digital".to_string());
                if let Ok(ac3) = Ac3Header::parse(&stream.head) {
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
                report.audios.push(a);
            }
            // Private stream 1: DTS
            0x188..=0x18F => {
                let mut a = AudioTrack::default();
                a.stream_id = stream_id;
                a.codec_id = Some(format!("{:02X}", key & 0xFF));
                a.duration_ms = duration_ms;
                a.stream_size = Some(stream.bytes);
                a.bit_rate = bit_rate;
                a.format = AudioCodec::DTS;
                a.format_info = Some("DTS".to_string());
                if let Ok(dts) = DtsHeader::parse(&stream.head) {
                    a.sampling_rate = dts.sample_rate;
                    a.channels = dts.channels;
                    a.channel_layout = Some(dts.channel_layout);
                    a.bit_rate = Some(dts.bit_rate);
                    a.format_profile = Some(dts.profile_name.to_string());
                }
                report.audios.push(a);
            }
            // Private stream 1: LPCM
            0x1A0..=0x1AF => {
                let mut a = AudioTrack::default();
                a.stream_id = stream_id;
                a.codec_id = Some(format!("{:02X}", key & 0xFF));
                a.duration_ms = duration_ms;
                a.stream_size = Some(stream.bytes);
                a.bit_rate = bit_rate;
                a.format = AudioCodec::PCM;
                a.format_info = Some("Linear PCM".to_string());
                a.compression_mode = Some("Lossless".to_string());
                report.audios.push(a);
            }
            // Private stream 1: DVD subpictures
            0x120..=0x13F => {
                let mut t = TextTrack::default();
                t.stream_id = stream_id;
                t.codec_id = Some(format!("{:02X}", key & 0xFF));
                t.format = SubtitleCodec::VobSub;
                t.format_info = Some("DVD Subtitle".to_string());
                report.texts.push(t);
            }
            _ => {}
        }
    }

    /// Scans an elementary stream for an SPS NAL unit (type 7).
    fn find_avc_sps(data: &[u8]) -> Option<AvcSps> {
        let mut i = 0;
        while i + 4 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                let nal_type = data[i + 3] & 0x1F;
                if nal_type == 7 {
                    return AvcSps::parse(&data[i + 3..]).ok();
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        None
    }
}
