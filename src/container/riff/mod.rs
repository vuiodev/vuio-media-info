use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// State collected while walking an AVI chunk tree.
#[derive(Default)]
struct AviCtx {
    fps: Option<f64>,
    total_frames: Option<u32>,
    width: u32,
    height: u32,
    stream_type: [u8; 4],
    handler: [u8; 4],
    stream_rate: Option<f64>,
    stream_length: Option<u32>,
    stream_format: Option<Vec<u8>>,
    /// Payload bytes per AVI stream number, indexed by the chunk id prefix.
    stream_bytes: [u64; 16],
    /// First coded frame per stream, for codecs whose profile is only in the bitstream.
    first_frame: Vec<(usize, Vec<u8>)>,
    /// Stream number each built track came from, parallel to `videos` then `audios`.
    video_streams: Vec<usize>,
    audio_streams: Vec<usize>,
    next_stream: usize,
    videos: Vec<VideoTrack>,
    audios: Vec<AudioTrack>,
}

impl AviCtx {
    fn begin_stream(&mut self) {
        self.stream_type = [0; 4];
        self.handler = [0; 4];
        self.stream_rate = None;
        self.stream_length = None;
        self.stream_format = None;
    }

    fn end_stream(&mut self) {
        RiffDemuxer::finish_avi_stream(self);
        self.next_stream += 1;
    }
}

/// RIFF (AVI and WAV, including RF64 / BW64) container demuxer.
pub struct RiffDemuxer;

impl RiffDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        let magic = &data[0..4];
        let is_riff_family =
            magic == b"RIFF" || magic == b"RIFX" || magic == b"RF64" || magic == b"BW64";

        if !is_riff_family {
            return Err(MediaInfoError::InvalidData(
                "Not a valid RIFF/RF64 file".to_string(),
            ));
        }

        let form_type = &data[8..12];
        let mut report = MediaReport::new();
        // RF64 / BW64 are the 64-bit-size variants of Wave.
        if magic == b"RF64" || magic == b"BW64" {
            report.general.format_profile = Some(String::from_utf8_lossy(magic).to_string());
        }
        report.general.file_size = data.len() as u64;

        if form_type == b"WAVE" {
            report.general.format = ContainerFormat::WAV;
            Self::parse_wav(data, &mut report)?;
        } else if form_type == b"AVI " || form_type == b"AVIX" {
            report.general.format = ContainerFormat::AVI;
            Self::parse_avi(data, &mut report)?;
        }

        Ok(report)
    }

    fn parse_wav(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let mut offset = 12;
        let mut audio_track = AudioTrack::default();
        audio_track.format = AudioCodec::PCM;
        audio_track.format_info = Some("Pulse Code Modulation".to_string());
        let mut data_chunk_size = 0u64;

        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;

            let payload_offset = offset + 8;
            let payload_size = if payload_offset + chunk_size <= data.len() {
                chunk_size
            } else {
                data.len().saturating_sub(payload_offset)
            };

            let payload = &data[payload_offset..payload_offset + payload_size];

            if chunk_id == b"ds64" && payload.len() >= 16 {
                // RF64 64-bit size chunk
                let ds_data_size = u64::from_le_bytes([
                    payload[8],
                    payload[9],
                    payload[10],
                    payload[11],
                    payload[12],
                    payload[13],
                    payload[14],
                    payload[15],
                ]);
                if ds_data_size > 0 {
                    data_chunk_size = ds_data_size;
                }
            } else if chunk_id == b"fmt " && payload.len() >= 16 {
                let format_tag = u16::from_le_bytes([payload[0], payload[1]]);
                let channels = u16::from_le_bytes([payload[2], payload[3]]) as u32;
                let sample_rate =
                    u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let byte_rate =
                    u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
                let _block_align = u16::from_le_bytes([payload[12], payload[13]]);
                let mut bit_depth = u16::from_le_bytes([payload[14], payload[15]]) as u8;

                audio_track.channels = channels;
                audio_track.sampling_rate = sample_rate;
                audio_track.bit_rate = Some(byte_rate as u64 * 8);

                let mut channel_layout = match channels {
                    1 => AudioChannelLayout::Mono,
                    2 => AudioChannelLayout::Stereo,
                    6 => AudioChannelLayout::Surround5_1,
                    8 => AudioChannelLayout::Surround7_1,
                    _ => AudioChannelLayout::Stereo,
                };

                if format_tag == 0xFFFE && payload.len() >= 40 {
                    // WAVE_FORMAT_EXTENSIBLE
                    let valid_bits = u16::from_le_bytes([payload[18], payload[19]]) as u8;
                    if valid_bits > 0 {
                        bit_depth = valid_bits;
                    }
                    let channel_mask =
                        u32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]);
                    if (channel_mask & 0x003F) == 0x003F {
                        channel_layout = AudioChannelLayout::Surround5_1;
                    } else if (channel_mask & 0x00FF) == 0x00FF || (channel_mask & 0x063F) == 0x063F
                    {
                        channel_layout = AudioChannelLayout::Surround7_1;
                    } else if channel_mask == 0x0004 {
                        channel_layout = AudioChannelLayout::Mono;
                    } else if channel_mask == 0x0003 {
                        channel_layout = AudioChannelLayout::Stereo;
                    }

                    let subformat_guid_code =
                        u32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]);
                    match subformat_guid_code {
                        0x0003 => {
                            audio_track.format = AudioCodec::PCM;
                            audio_track.format_info = Some("IEEE Float (Extensible)".to_string());
                        }
                        0x0092 => {
                            audio_track.format = AudioCodec::AC3;
                            audio_track.format_info = Some("Dolby Digital (AC-3)".to_string());
                        }
                        _ => {
                            audio_track.format = AudioCodec::PCM;
                            audio_track.format_info = Some("PCM (Extensible)".to_string());
                        }
                    }
                } else if format_tag == 3 {
                    audio_track.format = AudioCodec::PCM;
                    audio_track.format_info = Some("IEEE Float".to_string());
                } else if format_tag == 0x0055 {
                    audio_track.format = AudioCodec::MPEGAudioLayer3;
                    audio_track.format_info = Some("MPEG Audio Layer 3".to_string());
                }

                audio_track.bit_depth = Some(bit_depth);
                audio_track.codec_id = Some(if format_tag == 0xFFFE && payload.len() >= 40 {
                    // Extensible headers identify the real format by a 16-byte SubFormat GUID.
                    let g = &payload[24..40];
                    format!(
                        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{}",
                        u32::from_le_bytes([g[0], g[1], g[2], g[3]]),
                        u16::from_le_bytes([g[4], g[5]]),
                        u16::from_le_bytes([g[6], g[7]]),
                        g[8],
                        g[9],
                        g[10..16]
                            .iter()
                            .map(|b| format!("{b:02X}"))
                            .collect::<String>()
                    )
                } else {
                    format_tag.to_string()
                });
                if audio_track.format == AudioCodec::PCM && audio_track.format_profile.is_none() {
                    audio_track.format_profile = Some(
                        match format_tag {
                            3 => "Float",
                            _ => "Little / Signed",
                        }
                        .to_string(),
                    );
                }
                audio_track.channel_layout = Some(channel_layout);
                audio_track.compression_mode = Some(
                    if format_tag == 1 || format_tag == 3 || format_tag == 0xFFFE {
                        "Lossless".to_string()
                    } else {
                        "Lossy".to_string()
                    },
                );
            } else if chunk_id == b"bext" && payload.len() >= 346 {
                Self::parse_bext_chunk(payload, report);
            } else if (chunk_id == b"iXML" || chunk_id == b"ixml") && !payload.is_empty() {
                Self::parse_ixml_chunk(payload, report);
            } else if chunk_id == b"LIST" && payload.len() >= 4 {
                if &payload[0..4] == b"INFO" {
                    Self::parse_riff_info_list(&payload[4..], report);
                }
            } else if chunk_id == b"data" {
                if data_chunk_size == 0 {
                    data_chunk_size = chunk_size as u64;
                }
            }

            // Move to next chunk (aligned to 2 bytes)
            offset = payload_offset + chunk_size + (chunk_size % 2);
        }

        if let Some(bitrate) = audio_track.bit_rate {
            if bitrate > 0 && data_chunk_size > 0 {
                let duration_ms = ((data_chunk_size * 8) as f64 / bitrate as f64) * 1000.0;
                audio_track.duration_ms = Some(duration_ms);
                report.general.duration_ms = Some(duration_ms);
                report.general.overall_bitrate = Some(bitrate);
            }
        }

        report.audios.push(audio_track);
        Ok(())
    }

    fn parse_avi(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let mut ctx = AviCtx::default();
        Self::walk_avi_list(&data[12..], report, &mut ctx, 0);

        // Frame count and rate come from the main header; the per-stream headers refine them.
        if let Some(fps) = ctx.fps {
            if let Some(frames) = ctx.total_frames {
                if fps > 0.0 && frames > 0 {
                    let duration_ms = (frames as f64 / fps) * 1000.0;
                    report.general.duration_ms = Some(duration_ms);
                }
            }
        }
        let duration_ms = report.general.duration_ms;
        if let Some(ms) = duration_ms {
            if ms > 0.0 {
                report.general.overall_bitrate =
                    Some(((report.general.file_size * 8) as f64 / (ms / 1000.0)) as u64);
            }
        }

        for (idx, mut v) in ctx.videos.iter().cloned().enumerate() {
            if let Some(&n) = ctx.video_streams.get(idx) {
                if v.format == VideoCodec::MPEG4Visual && v.format_profile.is_none() {
                    if let Some((_, frame)) = ctx.first_frame.iter().find(|(sn, _)| *sn == n) {
                        v.format_profile =
                            crate::video::mpeg4_visual_profile(frame).map(str::to_string);
                    }
                }
                let bytes = ctx.stream_bytes.get(n).copied().unwrap_or(0);
                if bytes > 0 {
                    v.stream_size = Some(bytes);
                    if let Some(ms) = duration_ms {
                        if ms > 0.0 {
                            v.bit_rate = Some((bytes as f64 * 8.0 / (ms / 1000.0)) as u64);
                        }
                    }
                }
            }
            if v.width == 0 || v.height == 0 {
                v.width = ctx.width;
                v.height = ctx.height;
            }
            if v.frame_rate.is_none() {
                v.frame_rate = ctx.fps;
            }
            v.duration_ms = duration_ms;
            if v.color_space.is_none() {
                v.color_space = Some("YUV".to_string());
            }
            if v.width > 0 && v.height > 0 {
                v.display_aspect_ratio = Some(v.width as f64 / v.height as f64);
            }
            report.videos.push(v);
        }
        for (idx, mut a) in ctx.audios.iter().cloned().enumerate() {
            if let Some(&n) = ctx.audio_streams.get(idx) {
                let bytes = ctx.stream_bytes.get(n).copied().unwrap_or(0);
                if bytes > 0 {
                    a.stream_size = Some(bytes);
                    if a.bit_rate.is_none() {
                        if let Some(ms) = duration_ms {
                            if ms > 0.0 {
                                a.bit_rate = Some((bytes as f64 * 8.0 / (ms / 1000.0)) as u64);
                            }
                        }
                    }
                }
            }
            a.duration_ms = duration_ms;
            report.audios.push(a);
        }

        Ok(())
    }

    /// Walks an AVI chunk list, descending into `LIST` containers.
    ///
    /// `avih`, `strh` and `strf` live inside `LIST hdrl` / `LIST strl`, so a flat scan of
    /// the top level never reaches any of them.
    fn walk_avi_list(data: &[u8], report: &mut MediaReport, ctx: &mut AviCtx, depth: u8) {
        if depth > 4 {
            return;
        }
        let mut offset = 0;
        while offset + 8 <= data.len() {
            let chunk_id: [u8; 4] = match data[offset..offset + 4].try_into() {
                Ok(id) => id,
                Err(_) => break,
            };
            let chunk_size = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            let payload_offset = offset + 8;
            let end = payload_offset.saturating_add(chunk_size).min(data.len());
            if payload_offset > end {
                break;
            }
            let payload = &data[payload_offset..end];

            match &chunk_id {
                b"LIST" if payload.len() >= 4 => {
                    let list_type = &payload[0..4];
                    match list_type {
                        b"INFO" => Self::parse_riff_info_list(&payload[4..], report),
                        b"movi" => Self::accumulate_movi_sizes(&payload[4..], ctx),
                        b"strl" => {
                            ctx.begin_stream();
                            Self::walk_avi_list(&payload[4..], report, ctx, depth + 1);
                            ctx.end_stream();
                        }
                        _ => Self::walk_avi_list(&payload[4..], report, ctx, depth + 1),
                    }
                }
                b"avih" if payload.len() >= 40 => {
                    let microsec_per_frame =
                        u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    if microsec_per_frame > 0 {
                        ctx.fps = Some(1_000_000.0 / microsec_per_frame as f64);
                    }
                    let total_frames =
                        u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);
                    if total_frames > 0 {
                        ctx.total_frames = Some(total_frames);
                    }
                    ctx.width =
                        u32::from_le_bytes([payload[32], payload[33], payload[34], payload[35]]);
                    ctx.height =
                        u32::from_le_bytes([payload[36], payload[37], payload[38], payload[39]]);
                }
                b"strh" if payload.len() >= 40 => {
                    ctx.stream_type = payload[0..4].try_into().unwrap_or([0; 4]);
                    ctx.handler = payload[4..8].try_into().unwrap_or([0; 4]);
                    let scale =
                        u32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]);
                    let rate =
                        u32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]);
                    if scale > 0 && rate > 0 {
                        ctx.stream_rate = Some(rate as f64 / scale as f64);
                    }
                    let length =
                        u32::from_le_bytes([payload[32], payload[33], payload[34], payload[35]]);
                    ctx.stream_length = Some(length);
                }
                b"strf" => ctx.stream_format = Some(payload.to_vec()),
                b"bext" if payload.len() >= 346 => Self::parse_bext_chunk(payload, report),
                _ => {}
            }

            // Chunks are padded to an even byte boundary.
            offset = payload_offset + chunk_size + (chunk_size & 1);
            if chunk_size == 0 && &chunk_id != b"LIST" {
                offset = offset.max(payload_offset + 1);
            }
        }
    }

    /// Sums the payload bytes of each `##wb` / `##dc` chunk in the movi list, keyed by
    /// the two-digit stream number that prefixes the chunk id.
    fn accumulate_movi_sizes(data: &[u8], ctx: &mut AviCtx) {
        let mut offset = 0;
        let is_large = data.len() > 8 * 1024 * 1024;
        let total_streams = ctx.video_streams.len() + ctx.audio_streams.len();
        while offset + 8 <= data.len() {
            let id = &data[offset..offset + 4];
            let size = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            if id[0].is_ascii_digit() && id[1].is_ascii_digit() {
                let stream = ((id[0] - b'0') * 10 + (id[1] - b'0')) as usize;
                if stream < 16 {
                    ctx.stream_bytes[stream] += size as u64;
                    if size > 0
                        && !ctx.first_frame.iter().any(|(n, _)| *n == stream)
                        && offset + 8 + size <= data.len()
                    {
                        let head = &data[offset + 8..offset + 8 + size.min(4096)];
                        ctx.first_frame.push((stream, head.to_vec()));
                    }
                }
            }
            if is_large
                && total_streams > 0
                && ctx.first_frame.len() >= total_streams
                && offset > 4 * 1024 * 1024
            {
                break;
            }
            offset += 8 + size + (size & 1);
        }
    }

    /// Builds a track from the `strh` / `strf` pair collected for one `LIST strl`.
    fn finish_avi_stream(ctx: &mut AviCtx) {
        let Some(format) = ctx.stream_format.take() else {
            return;
        };
        let stream_id = ctx.videos.len() as u32 + ctx.audios.len() as u32 + 1;
        let stream_number = ctx.next_stream;

        match &ctx.stream_type {
            b"vids" if format.len() >= 40 => {
                // strf is a BITMAPINFOHEADER.
                let mut v = VideoTrack::default();
                v.stream_id = stream_id;
                v.width = u32::from_le_bytes([format[4], format[5], format[6], format[7]]);
                v.height = i32::from_le_bytes([format[8], format[9], format[10], format[11]])
                    .unsigned_abs();
                let bit_count = u16::from_le_bytes([format[14], format[15]]);
                let compression: [u8; 4] = format[16..20].try_into().unwrap_or([0; 4]);

                // An empty biCompression means uncompressed RGB.
                let fourcc = if compression == [0, 0, 0, 0] {
                    *b"RGB "
                } else {
                    compression
                };
                let (codec, info) = Self::avi_video_codec(&fourcc);
                v.format = codec;
                v.format_info = info.map(str::to_string);
                v.codec_id = Some(
                    String::from_utf8_lossy(&ctx.handler)
                        .trim()
                        .trim_end_matches('\0')
                        .to_string(),
                );
                if compression == [0, 0, 0, 0] {
                    v.chroma_subsampling = Some(ChromaSubsampling::RGB);
                    v.color_space = Some("RGB".to_string());
                    v.compression_mode = Some("Lossless".to_string());
                    if bit_count > 0 {
                        v.bit_depth = (bit_count / 3).clamp(8, 16) as u8;
                    }
                }
                // DV's chroma format depends on the line system, so it is left to the
                // shared finalize step which sees the final frame geometry.
                v.chroma_subsampling = v.chroma_subsampling.or(match v.format {
                    VideoCodec::MPEG4Visual | VideoCodec::MPEG2Video => {
                        Some(ChromaSubsampling::YUV420)
                    }
                    _ => None,
                });
                v.frame_rate = ctx.stream_rate;
                v.frame_count = ctx.stream_length.map(|l| l as u64);
                ctx.videos.push(v);
                ctx.video_streams.push(stream_number);
            }
            b"auds" if format.len() >= 16 => {
                // strf is a WAVEFORMATEX.
                let mut a = AudioTrack::default();
                a.stream_id = stream_id;
                let format_tag = u16::from_le_bytes([format[0], format[1]]);
                a.channels = u16::from_le_bytes([format[2], format[3]]) as u32;
                a.sampling_rate = u32::from_le_bytes([format[4], format[5], format[6], format[7]]);
                let byte_rate = u32::from_le_bytes([format[8], format[9], format[10], format[11]]);
                let bits = u16::from_le_bytes([format[14], format[15]]) as u8;
                if byte_rate > 0 {
                    a.bit_rate = Some(byte_rate as u64 * 8);
                }
                if bits > 0 {
                    a.bit_depth = Some(bits);
                }
                a.channel_layout = AudioChannelLayout::from_channel_count(a.channels);
                let (codec, info) = Self::wave_format_codec(format_tag);
                a.format = codec;
                a.format_info = info.map(str::to_string);
                a.codec_id = Some(format!("{format_tag:X}"));
                a.compression_mode = Some(
                    if matches!(format_tag, 1 | 3 | 0xFFFE) {
                        "Lossless"
                    } else {
                        "Lossy"
                    }
                    .to_string(),
                );
                ctx.audios.push(a);
                ctx.audio_streams.push(stream_number);
            }
            _ => {}
        }
    }

    fn avi_video_codec(fourcc: &[u8; 4]) -> (VideoCodec, Option<&'static str>) {
        let upper: [u8; 4] = [
            fourcc[0].to_ascii_uppercase(),
            fourcc[1].to_ascii_uppercase(),
            fourcc[2].to_ascii_uppercase(),
            fourcc[3].to_ascii_uppercase(),
        ];
        match &upper {
            b"H264" | b"X264" | b"AVC1" => (VideoCodec::AVC, Some("Advanced Video Coding")),
            b"HEVC" | b"H265" | b"X265" => (VideoCodec::HEVC, Some("High Efficiency Video Coding")),
            b"XVID" | b"DIVX" | b"DX50" | b"FMP4" | b"MP4V" => {
                (VideoCodec::MPEG4Visual, Some("MPEG-4 Visual"))
            }
            b"MPG2" | b"MPEG" | b"MP2V" => (VideoCodec::MPEG2Video, Some("MPEG-2 Video")),
            b"DVSD" | b"DVC " | b"DVCP" | b"DV25" | b"DV50" | b"CDVC" => {
                (VideoCodec::DV, Some("Digital Video"))
            }
            b"MJPG" => (VideoCodec::Other("JPEG".to_string()), Some("Motion JPEG")),
            b"FFV1" => (VideoCodec::FFV1, Some("FFmpeg Video 1")),
            b"CFHD" => (VideoCodec::CineForm, Some("GoPro CineForm")),
            b"AVDN" | b"AVDH" => (VideoCodec::DNxHD, Some("Avid DNxHD / DNxHR")),
            b"VP80" => (VideoCodec::VP8, Some("Google VP8")),
            b"VP90" => (VideoCodec::VP9, Some("Google VP9")),
            b"WMV3" | b"WVC1" => (VideoCodec::VC1, Some("SMPTE 421M")),
            b"RGB " => (VideoCodec::Raw, Some("Uncompressed RGB")),
            b"UYVY" | b"YUY2" | b"YV12" | b"I420" => (VideoCodec::Raw, Some("Uncompressed")),
            _ => (
                VideoCodec::Other(
                    String::from_utf8_lossy(fourcc)
                        .trim()
                        .trim_end_matches('\0')
                        .to_string(),
                ),
                None,
            ),
        }
    }

    /// Maps a WAVE format tag to a codec.
    fn wave_format_codec(tag: u16) -> (AudioCodec, Option<&'static str>) {
        match tag {
            0x0001 => (AudioCodec::PCM, Some("Integer PCM")),
            0x0002 => (AudioCodec::Other("ADPCM".to_string()), Some("MS ADPCM")),
            0x0003 => (AudioCodec::PCM, Some("IEEE Float")),
            0x0006 => (AudioCodec::Other("A-law".to_string()), Some("ITU G.711")),
            0x0007 => (AudioCodec::Other("mu-law".to_string()), Some("ITU G.711")),
            0x0011 => (AudioCodec::Other("ADPCM".to_string()), Some("IMA ADPCM")),
            0x0050 => (AudioCodec::MPEGAudioLayer2, Some("MPEG Audio")),
            0x0055 => (AudioCodec::MPEGAudioLayer3, Some("MPEG Audio Layer 3")),
            0x0092 | 0x2000 => (AudioCodec::AC3, Some("Dolby Digital")),
            0x2001 => (AudioCodec::DTS, Some("DTS")),
            0x00FF | 0x1600 | 0x1601 => (AudioCodec::AAC, Some("Advanced Audio Coding")),
            0x0160 => (AudioCodec::WMA, Some("Windows Media Audio 1")),
            0x0161 => (AudioCodec::WMA, Some("Windows Media Audio 2")),
            0x0162 => (AudioCodec::WMA, Some("Windows Media Audio Professional")),
            0x0163 => (AudioCodec::WMA, Some("Windows Media Audio Lossless")),
            0x674F..=0x6751 => (AudioCodec::Vorbis, Some("Vorbis")),
            0xF1AC => (AudioCodec::FLAC, Some("Free Lossless Audio Codec")),
            0xFFFE => (AudioCodec::PCM, Some("PCM (Extensible)")),
            _ => (AudioCodec::Other(format!("0x{tag:04X}")), None),
        }
    }

    fn parse_riff_info_list(mut info_data: &[u8], report: &mut MediaReport) {
        while info_data.len() >= 8 {
            let fourcc = &info_data[0..4];
            let chunk_size =
                u32::from_le_bytes([info_data[4], info_data[5], info_data[6], info_data[7]])
                    as usize;

            let payload_offset = 8;
            if payload_offset + chunk_size > info_data.len() {
                break;
            }

            let text_bytes = &info_data[payload_offset..payload_offset + chunk_size];
            let clean_str = String::from_utf8_lossy(text_bytes)
                .trim_end_matches('\0')
                .trim()
                .to_string();

            if !clean_str.is_empty() {
                match fourcc {
                    b"INAM" => report.general.title = Some(clean_str),
                    b"IART" => report.general.artist = Some(clean_str),
                    b"IPRD" | b"IALB" => report.general.album = Some(clean_str),
                    b"ICRD" => report.general.recorded_date = Some(clean_str),
                    b"IGNR" => report.general.genre = Some(clean_str),
                    b"ISFT" | b"IENG" => report.general.encoded_application = Some(clean_str),
                    _ => {}
                }
            }

            let next_offset = payload_offset + chunk_size + (chunk_size % 2);
            if next_offset >= info_data.len() {
                break;
            }
            info_data = &info_data[next_offset..];
        }
    }

    fn parse_bext_chunk(payload: &[u8], report: &mut MediaReport) {
        if payload.len() < 346 {
            return;
        }
        let desc = String::from_utf8_lossy(&payload[0..256])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let orig = String::from_utf8_lossy(&payload[256..288])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let orig_ref = String::from_utf8_lossy(&payload[288..320])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let orig_date = String::from_utf8_lossy(&payload[320..330])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let orig_time = String::from_utf8_lossy(&payload[330..338])
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let time_ref = u64::from_le_bytes([
            payload[338],
            payload[339],
            payload[340],
            payload[341],
            payload[342],
            payload[343],
            payload[344],
            payload[345],
        ]);

        if !desc.is_empty() && report.general.title.is_none() {
            report.general.title = Some(desc.clone());
        }
        if !orig.is_empty() && report.general.encoded_application.is_none() {
            report.general.encoded_application = Some(orig.clone());
        }
        if !orig_date.is_empty() {
            report.general.recorded_date = Some(if !orig_time.is_empty() {
                format!("{} {}", orig_date, orig_time)
            } else {
                orig_date
            });
        }

        report
            .general
            .extra
            .insert("BWF:Description".to_string(), desc);
        report
            .general
            .extra
            .insert("BWF:Originator".to_string(), orig);
        report
            .general
            .extra
            .insert("BWF:OriginatorReference".to_string(), orig_ref);
        report
            .general
            .extra
            .insert("BWF:TimeReference".to_string(), time_ref.to_string());

        // BWF version 1/2 loudness metadata (offset 412..422)
        if payload.len() >= 422 {
            let loudness_val = i16::from_le_bytes([payload[412], payload[413]]) as f64 / 100.0;
            let loudness_range = i16::from_le_bytes([payload[414], payload[415]]) as f64 / 100.0;
            let max_true_peak = i16::from_le_bytes([payload[416], payload[417]]) as f64 / 100.0;
            let max_momentary = i16::from_le_bytes([payload[418], payload[419]]) as f64 / 100.0;
            let max_short_term = i16::from_le_bytes([payload[420], payload[421]]) as f64 / 100.0;

            if loudness_val != 0.0 {
                report.general.extra.insert(
                    "EBU R128:IntegratedLoudness".to_string(),
                    format!("{:.2} LUFS", loudness_val),
                );
            }
            if loudness_range != 0.0 {
                report.general.extra.insert(
                    "EBU R128:LoudnessRange".to_string(),
                    format!("{:.2} LU", loudness_range),
                );
            }
            if max_true_peak != 0.0 {
                report.general.extra.insert(
                    "EBU R128:MaxTruePeak".to_string(),
                    format!("{:.2} dBFS", max_true_peak),
                );
            }
            if max_momentary != 0.0 {
                report.general.extra.insert(
                    "EBU R128:MaxMomentaryLoudness".to_string(),
                    format!("{:.2} LUFS", max_momentary),
                );
            }
            if max_short_term != 0.0 {
                report.general.extra.insert(
                    "EBU R128:MaxShortTermLoudness".to_string(),
                    format!("{:.2} LUFS", max_short_term),
                );
            }
        }
    }

    fn parse_ixml_chunk(payload: &[u8], report: &mut MediaReport) {
        let xml_str = String::from_utf8_lossy(payload);
        // Extract simple tags via pattern search without bulky XML parser
        let extract_tag = |tag: &str| -> Option<String> {
            let open = format!("<{}>", tag);
            let close = format!("</{}>", tag);
            if let (Some(start), Some(end)) = (xml_str.find(&open), xml_str.find(&close)) {
                let val = &xml_str[start + open.len()..end].trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
            None
        };

        if let Some(project) = extract_tag("PROJECT") {
            report
                .general
                .extra
                .insert("iXML:Project".to_string(), project);
        }
        if let Some(scene) = extract_tag("SCENE") {
            report.general.extra.insert("iXML:Scene".to_string(), scene);
        }
        if let Some(take) = extract_tag("TAKE") {
            report.general.extra.insert("iXML:Take".to_string(), take);
        }
        if let Some(tape) = extract_tag("TAPE") {
            report.general.extra.insert("iXML:Tape".to_string(), tape);
        }
        if let Some(notes) = extract_tag("NOTE") {
            report.general.extra.insert("iXML:Notes".to_string(), notes);
        }
    }
}
