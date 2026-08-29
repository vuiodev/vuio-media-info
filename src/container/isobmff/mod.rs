use crate::audio::AacInfo;
use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use crate::tags::ItunesTags;
use crate::video::{AvcSps, CineFormHeader, HevcSps, ProResHeader, ProResVariant};
use std::ops::Range;

/// Per-track state gathered from `tkhd`, `mdhd` and the sample tables before the
/// sample description is decoded.
#[derive(Debug, Default, Clone)]
struct TrackCtx {
    stream_id: u32,
    handler: [u8; 4],
    width: u32,
    height: u32,
    timescale: u32,
    duration: u64,
    duration_ms: Option<f64>,
    language: Option<String>,
    enabled: bool,
    frame_rate: Option<f64>,
    frame_rate_mode: Option<FrameRateMode>,
    sample_count: u64,
    stream_size: Option<u64>,
    bit_rate: Option<u64>,
    /// Byte range of the first sample in the file, for bitstream-level probing.
    first_sample: Option<Range<usize>>,
}

#[derive(Debug, Default, Clone, Copy)]
struct SampleSizes {
    total: u64,
    count: u32,
    first: Option<u32>,
}

/// MP4 / QuickTime / ISOBMFF container demuxer.
pub struct IsobmffDemuxer;

impl IsobmffDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MPEG4;
        report.general.file_size = data.len() as u64;

        let mut root_node = BitstreamNode::new("root", 0, data.len() as u64);
        let mut offset = 0usize;

        while offset + 8 <= data.len() {
            let (box_type, box_size, header_len) = Self::read_box_header(data, offset)?;
            let box_type_str = String::from_utf8_lossy(&box_type).to_string();
            let box_node = BitstreamNode::new(&box_type_str, offset as u64, box_size as u64);

            let payload_offset = offset + header_len;
            let payload_size = box_size.saturating_sub(header_len);
            let payload = if payload_offset + payload_size <= data.len() {
                &data[payload_offset..payload_offset + payload_size]
            } else {
                &data[payload_offset..]
            };

            match &box_type {
                b"ftyp" => {
                    Self::parse_ftyp(payload, &mut report);
                }
                b"moov" => {
                    Self::parse_moov(payload, &mut report, data);
                    if !report.videos.is_empty() || !report.audios.is_empty() {
                        break;
                    }
                }
                _ => {}
            }

            root_node.add_child(box_node);
            offset += box_size;
            if box_size == 0 {
                break;
            }
        }

        report.bitstream_root = Some(root_node);
        Ok(report)
    }

    fn read_box_header(data: &[u8], offset: usize) -> Result<([u8; 4], usize, usize)> {
        if offset + 8 > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len() - offset,
            });
        }

        let size32 = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        let box_type = [
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ];

        if size32 == 1 {
            if offset + 16 > data.len() {
                return Err(MediaInfoError::UnexpectedEof {
                    expected: 16,
                    actual: data.len() - offset,
                });
            }
            let size64 = u64::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]) as usize;
            Ok((box_type, size64, 16))
        } else if size32 == 0 {
            Ok((box_type, data.len() - offset, 8))
        } else {
            Ok((box_type, size32, 8))
        }
    }

    fn parse_ftyp(data: &[u8], report: &mut MediaReport) {
        if data.len() < 8 {
            return;
        }

        let major_brand = String::from_utf8_lossy(&data[0..4]).to_string();
        let minor_version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let mut compat_brands = Vec::new();
        for chunk in data[8..].as_chunks::<4>().0 {
            let brand = String::from_utf8_lossy(chunk).trim().to_string();
            if !brand.is_empty() {
                compat_brands.push(brand);
            }
        }

        report.general.codec_id = Some(major_brand.clone());
        report.general.format_profile = Some(Self::brand_name(major_brand.trim()).to_string());
        report.general.format_version = Some(minor_version.to_string());
        if !compat_brands.is_empty() {
            report
                .general
                .extra
                .insert("CompatibleBrands".to_string(), compat_brands.join(" "));
        }
    }

    /// Human-readable name for an ISOBMFF major brand.
    fn brand_name(brand: &str) -> &str {
        match brand {
            "qt" => "QuickTime",
            "isom" => "Base Media",
            "iso2" => "Base Media / Version 2",
            "iso4" => "Base Media / Version 4",
            "iso5" => "Base Media / Version 5",
            "iso6" => "Base Media / Version 6",
            "mp41" => "Base Media / Version 1",
            "mp42" => "Base Media / Version 2",
            "M4A" => "Apple audio with iTunes info",
            "M4B" => "Apple audiobook",
            "M4V" => "Apple video with iTunes info",
            "M4P" => "Apple protected audio",
            "avc1" => "JVT / AVC",
            "dash" => "MPEG-DASH",
            "3gp4" | "3gp5" | "3gp6" | "3gp7" => "3GPP Media",
            "heic" | "heix" => "High Efficiency Image Format",
            "avif" => "AV1 Image File Format",
            other => other,
        }
    }

    fn parse_moov(data: &[u8], report: &mut MediaReport, file: &[u8]) {
        let mut offset = 0;
        let mut track_id_counter = 1u32;

        while offset + 8 <= data.len() {
            if let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) {
                let payload_offset = offset + header_len;
                let payload_size = box_size.saturating_sub(header_len);
                let payload = if payload_offset + payload_size <= data.len() {
                    &data[payload_offset..payload_offset + payload_size]
                } else {
                    &data[payload_offset..]
                };

                match &box_type {
                    b"mvhd" => {
                        Self::parse_mvhd(payload, report);
                    }
                    b"trak" => {
                        Self::parse_trak(payload, track_id_counter, report, file);
                        track_id_counter += 1;
                    }
                    b"udta" => {
                        Self::parse_udta(payload, report);
                    }
                    _ => {}
                }

                offset += box_size;
                if box_size == 0 {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn parse_mvhd(data: &[u8], report: &mut MediaReport) {
        if data.len() < 20 {
            return;
        }

        let version = data[0];
        let (timescale, duration) = if version == 1 {
            if data.len() < 32 {
                return;
            }
            let ts = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as u64;
            let dur = u64::from_be_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]);
            (ts, dur)
        } else {
            let ts = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as u64;
            let dur = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as u64;
            (ts, dur)
        };

        if timescale > 0 {
            let duration_ms = (duration as f64 / timescale as f64) * 1000.0;
            report.general.duration_ms = Some(duration_ms);
            if duration_ms > 0.0 && report.general.file_size > 0 {
                report.general.overall_bitrate =
                    Some(((report.general.file_size * 8) as f64 / (duration_ms / 1000.0)) as u64);
            }
        }
    }

    fn parse_trak(data: &[u8], track_id: u32, report: &mut MediaReport, file: &[u8]) {
        let mut ctx = TrackCtx {
            stream_id: track_id,
            ..Default::default()
        };

        // tkhd carries the presentation size and the enabled flag; it always precedes mdia.
        let mut offset = 0;
        while offset + 8 <= data.len() {
            let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) else {
                break;
            };
            let payload_offset = offset + header_len;
            let payload_size = box_size.saturating_sub(header_len);
            let payload = if payload_offset + payload_size <= data.len() {
                &data[payload_offset..payload_offset + payload_size]
            } else {
                &data[payload_offset..]
            };

            match &box_type {
                b"tkhd" => Self::parse_tkhd(payload, &mut ctx),
                b"mdia" => Self::parse_mdia(payload, &mut ctx, report, file),
                _ => {}
            }

            offset += box_size;
            if box_size == 0 {
                break;
            }
        }
    }

    /// Track header: flags (enabled/in-movie) and the 16.16 fixed-point display size.
    fn parse_tkhd(data: &[u8], ctx: &mut TrackCtx) {
        if data.len() < 4 {
            return;
        }
        let version = data[0];
        let flags = u32::from_be_bytes([0, data[1], data[2], data[3]]);
        ctx.enabled = (flags & 0x01) != 0;

        // Fixed part before the 3x3 matrix differs between v0 (32-bit times) and v1 (64-bit).
        let (id_off, size_off) = if version == 1 { (20, 96) } else { (12, 84) };
        if data.len() >= id_off + 4 {
            let tid = u32::from_be_bytes([
                data[id_off],
                data[id_off + 1],
                data[id_off + 2],
                data[id_off + 3],
            ]);
            if tid > 0 {
                ctx.stream_id = tid;
            }
        }
        if data.len() >= size_off + 8 {
            // width/height are 16.16 fixed point; the integer part is the display size.
            ctx.width = u16::from_be_bytes([data[size_off], data[size_off + 1]]) as u32;
            ctx.height = u16::from_be_bytes([data[size_off + 4], data[size_off + 5]]) as u32;
        }
    }

    fn parse_mdia(data: &[u8], ctx: &mut TrackCtx, report: &mut MediaReport, file: &[u8]) {
        let mut offset = 0;

        while offset + 8 <= data.len() {
            let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) else {
                break;
            };
            let payload_offset = offset + header_len;
            let payload_size = box_size.saturating_sub(header_len);
            let payload = if payload_offset + payload_size <= data.len() {
                &data[payload_offset..payload_offset + payload_size]
            } else {
                &data[payload_offset..]
            };

            match &box_type {
                b"mdhd" => Self::parse_mdhd(payload, ctx),
                b"hdlr" if payload.len() >= 12 => {
                    ctx.handler = [payload[8], payload[9], payload[10], payload[11]];
                }
                b"minf" => Self::parse_minf(payload, ctx, report, file),
                _ => {}
            }

            offset += box_size;
            if box_size == 0 {
                break;
            }
        }
    }

    /// Media header: media timescale, media duration and the packed ISO-639-2/T language.
    fn parse_mdhd(data: &[u8], ctx: &mut TrackCtx) {
        if data.len() < 4 {
            return;
        }
        let version = data[0];
        let (timescale, duration, lang_off) = if version == 1 {
            if data.len() < 36 {
                return;
            }
            (
                u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
                u64::from_be_bytes([
                    data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
                ]),
                32,
            )
        } else {
            if data.len() < 24 {
                return;
            }
            (
                u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
                u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as u64,
                20,
            )
        };

        ctx.timescale = timescale;
        ctx.duration = duration;
        if timescale > 0 && duration > 0 {
            ctx.duration_ms = Some((duration as f64 / timescale as f64) * 1000.0);
        }

        if data.len() >= lang_off + 2 {
            // Three 5-bit values, each offset from 0x60, packed into 16 bits.
            let packed = u16::from_be_bytes([data[lang_off], data[lang_off + 1]]);
            let chars = [
                (((packed >> 10) & 0x1F) as u8) + 0x60,
                (((packed >> 5) & 0x1F) as u8) + 0x60,
                ((packed & 0x1F) as u8) + 0x60,
            ];
            if chars.iter().all(|c| c.is_ascii_lowercase()) {
                let lang = String::from_utf8_lossy(&chars).to_string();
                if lang != "und" {
                    ctx.language = Some(lang);
                }
            }
        }
    }

    fn parse_minf(data: &[u8], ctx: &mut TrackCtx, report: &mut MediaReport, file: &[u8]) {
        let mut offset = 0;
        while offset + 8 <= data.len() {
            let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) else {
                break;
            };
            let payload_offset = offset + header_len;
            let payload_size = box_size.saturating_sub(header_len);
            let payload = if payload_offset + payload_size <= data.len() {
                &data[payload_offset..payload_offset + payload_size]
            } else {
                &data[payload_offset..]
            };

            if &box_type == b"stbl" {
                Self::parse_stbl(payload, ctx, report, file);
                break;
            }

            offset += box_size;
            if box_size == 0 {
                break;
            }
        }
    }

    /// Sample table: collects every child box first, then derives frame rate, stream size
    /// and the first sample's file range before decoding the sample description.
    fn parse_stbl(data: &[u8], ctx: &mut TrackCtx, report: &mut MediaReport, file: &[u8]) {
        let mut stsd: Option<&[u8]> = None;
        let mut stts: Option<&[u8]> = None;
        let mut stsz: Option<&[u8]> = None;
        let mut chunk_offsets: Option<(&[u8], bool)> = None;

        let mut offset = 0;
        while offset + 8 <= data.len() {
            let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) else {
                break;
            };
            let payload_offset = offset + header_len;
            let payload_size = box_size.saturating_sub(header_len);
            let payload = if payload_offset + payload_size <= data.len() {
                &data[payload_offset..payload_offset + payload_size]
            } else {
                &data[payload_offset..]
            };

            match &box_type {
                b"stsd" => stsd = Some(payload),
                b"stts" => stts = Some(payload),
                b"stsz" | b"stz2" => stsz = Some(payload),
                b"stco" => chunk_offsets = Some((payload, false)),
                b"co64" => chunk_offsets = Some((payload, true)),
                _ => {}
            }

            offset += box_size;
            if box_size == 0 {
                break;
            }
        }

        if let Some(stts) = stts {
            Self::parse_stts(stts, ctx);
        }
        let sample_sizes = stsz.map(Self::parse_stsz).unwrap_or_default();
        ctx.stream_size = Some(sample_sizes.total);
        if ctx.sample_count == 0 {
            ctx.sample_count = sample_sizes.count as u64;
        }

        // Locate the first sample so bitstream-only codecs (ProRes) can read a real frame.
        if let (Some((co, is64)), Some(first_len)) = (chunk_offsets, sample_sizes.first) {
            if let Some(start) = Self::first_chunk_offset(co, is64) {
                let start = start as usize;
                let end = (start + first_len as usize).min(file.len());
                if start < end {
                    ctx.first_sample = Some(start..end);
                }
            }
        }

        // Per-track bit rate from the real sample payload size.
        if let (Some(size), Some(ms)) = (ctx.stream_size, ctx.duration_ms) {
            if ms > 0.0 && size > 0 {
                ctx.bit_rate = Some(((size * 8) as f64 / (ms / 1000.0)) as u64);
            }
        }

        let Some(stsd) = stsd else { return };
        if stsd.len() < 8 {
            return;
        }
        Self::parse_stsd(&stsd[8..], ctx, report, file);
    }

    /// Decoding-time-to-sample: total sample count and, for CFR streams, the frame rate.
    fn parse_stts(data: &[u8], ctx: &mut TrackCtx) {
        if data.len() < 8 {
            return;
        }
        let entry_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let mut total_samples = 0u64;
        let mut total_delta = 0u64;
        let mut distinct_deltas: Vec<u32> = Vec::new();

        for i in 0..entry_count {
            let off = 8 + i * 8;
            if off + 8 > data.len() {
                break;
            }
            let count =
                u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) as u64;
            let delta =
                u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]);
            total_samples += count;
            total_delta += count * delta as u64;
            if delta != 0 && !distinct_deltas.contains(&delta) {
                distinct_deltas.push(delta);
            }
        }

        ctx.sample_count = total_samples;
        if ctx.timescale > 0 && total_delta > 0 && total_samples > 0 {
            ctx.frame_rate = Some(total_samples as f64 * ctx.timescale as f64 / total_delta as f64);
            ctx.frame_rate_mode = Some(if distinct_deltas.len() <= 1 {
                FrameRateMode::Constant
            } else {
                FrameRateMode::Variable
            });
        }
    }

    fn parse_stsz(data: &[u8]) -> SampleSizes {
        let mut out = SampleSizes::default();
        if data.len() < 12 {
            return out;
        }
        let uniform = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let count = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        out.count = count;

        if uniform > 0 {
            out.total = uniform as u64 * count as u64;
            out.first = Some(uniform);
            return out;
        }
        for i in 0..count as usize {
            let off = 12 + i * 4;
            if off + 4 > data.len() {
                break;
            }
            let size = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            if i == 0 {
                out.first = Some(size);
            }
            out.total += size as u64;
        }
        out
    }

    fn first_chunk_offset(data: &[u8], is64: bool) -> Option<u64> {
        if data.len() < 8 {
            return None;
        }
        if u32::from_be_bytes([data[4], data[5], data[6], data[7]]) == 0 {
            return None;
        }
        if is64 {
            if data.len() < 16 {
                return None;
            }
            Some(u64::from_be_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]))
        } else {
            if data.len() < 12 {
                return None;
            }
            Some(u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as u64)
        }
    }

    /// Sample description: dispatches the first entry to the video/audio/subtitle builder.
    fn parse_stsd(entry_data: &[u8], ctx: &TrackCtx, report: &mut MediaReport, file: &[u8]) {
        if entry_data.len() < 8 {
            return;
        }
        let Ok((codec_box, _, _)) = Self::read_box_header(entry_data, 0) else {
            return;
        };
        let codec_str = String::from_utf8_lossy(&codec_box).trim().to_string();

        match &ctx.handler {
            b"vide" => {
                Self::build_video_track(entry_data, &codec_box, &codec_str, ctx, report, file)
            }
            b"soun" => Self::build_audio_track(entry_data, &codec_box, &codec_str, ctx, report),
            b"subt" | b"text" | b"sbtl" | b"clcp" => {
                let mut s = TextTrack::default();
                s.stream_id = ctx.stream_id;
                s.codec_id = Some(codec_str.clone());
                s.language = ctx.language.clone();
                s.duration_ms = ctx.duration_ms;
                s.default_flag = ctx.enabled;
                s.format = match &codec_box {
                    b"tx3g" => SubtitleCodec::Other("Timed Text".to_string()),
                    b"text" => SubtitleCodec::Other("QuickTime Text".to_string()),
                    b"wvtt" => SubtitleCodec::WebVTT,
                    b"stpp" => SubtitleCodec::TTML,
                    b"c608" => SubtitleCodec::EIA608,
                    b"c708" => SubtitleCodec::EIA708,
                    _ => SubtitleCodec::Other(codec_str),
                };
                report.texts.push(s);
            }
            _ => {}
        }
    }

    fn build_video_track(
        entry_data: &[u8],
        codec_box: &[u8; 4],
        codec_str: &str,
        ctx: &TrackCtx,
        report: &mut MediaReport,
        file: &[u8],
    ) {
        let mut v = VideoTrack::default();
        v.stream_id = ctx.stream_id;
        v.codec_id = Some(codec_str.to_string());
        v.width = ctx.width;
        v.height = ctx.height;
        v.duration_ms = ctx.duration_ms;
        v.frame_rate = ctx.frame_rate;
        v.frame_rate_mode = ctx.frame_rate_mode;
        v.frame_count = (ctx.sample_count > 0).then_some(ctx.sample_count);
        v.stream_size = ctx.stream_size;
        v.bit_rate = ctx.bit_rate;
        v.language = ctx.language.clone();
        v.default_flag = ctx.enabled;

        // VisualSampleEntry fixed fields: coded size at +32/+34, depth at +82.
        if entry_data.len() >= 84 {
            let coded_w = u16::from_be_bytes([entry_data[32], entry_data[33]]) as u32;
            let coded_h = u16::from_be_bytes([entry_data[34], entry_data[35]]) as u32;
            if coded_w > 0 && coded_h > 0 {
                v.stored_width = Some(coded_w);
                v.stored_height = Some(coded_h);
                if v.width == 0 || v.height == 0 {
                    v.width = coded_w;
                    v.height = coded_h;
                }
            }
            let name_len = entry_data[50] as usize;
            if name_len > 0 && name_len <= 31 {
                let name = String::from_utf8_lossy(&entry_data[51..51 + name_len])
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    v.encoded_library = Some(name);
                }
            }
        }

        let (codec, info) = Self::video_codec_from_fourcc(codec_box);
        v.format = codec.clone();
        v.format_info = info.map(str::to_string);

        if codec == VideoCodec::MPEG4Visual {
            v.chroma_subsampling = Some(ChromaSubsampling::YUV420);
            if let Some(esds) = Self::find_sample_entry_child(entry_data, b"esds") {
                if let Some(oti) = Self::esds_object_type(esds) {
                    v.codec_id = Some(format!("{codec_str}-{oti:X}"));
                }
                if let Some(cfg) = Self::esds_decoder_config(esds) {
                    v.format_profile = crate::video::mpeg4_visual_profile(cfg).map(str::to_string);
                }
            }
        }

        match codec {
            VideoCodec::AVC => Self::decode_avcc(entry_data, &mut v),
            VideoCodec::HEVC => Self::decode_hvcc(entry_data, &mut v),
            VideoCodec::AV1 => Self::decode_av1c(entry_data, &mut v),
            VideoCodec::VP9 | VideoCodec::VP8 => Self::decode_vpcc(entry_data, &mut v),
            VideoCodec::ProRes => Self::decode_prores(entry_data, codec_box, ctx, &mut v, file),
            VideoCodec::CineForm => Self::decode_cineform(ctx, &mut v, file),
            _ => {}
        }

        // colr overrides bitstream colorimetry when the container carries a tag.
        if let Some(colr) = Self::find_sample_entry_child(entry_data, b"colr") {
            Self::apply_colr(colr, &mut v);
        }
        if let Some(pasp) = Self::find_sample_entry_child(entry_data, b"pasp") {
            if pasp.len() >= 8 {
                let h = u32::from_be_bytes([pasp[0], pasp[1], pasp[2], pasp[3]]);
                let vv = u32::from_be_bytes([pasp[4], pasp[5], pasp[6], pasp[7]]);
                if h > 0 && vv > 0 {
                    v.sample_aspect_ratio = Some(h as f64 / vv as f64);
                }
            }
        }

        if v.color_space.is_none() && !matches!(v.format, VideoCodec::ProRes) {
            v.color_space = Some(
                match v.chroma_subsampling {
                    Some(ChromaSubsampling::RGB) => "RGB",
                    _ => "YUV",
                }
                .to_string(),
            );
        }
        if matches!(v.format, VideoCodec::ProRes)
            && v.color_space.is_none()
            && !ProResVariant::from_fourcc(codec_box).has_alpha()
        {
            v.color_space = Some("YUV".to_string());
        }

        // Broadcast standard, which MediaInfo derives from the frame geometry and rate.
        if v.standard.is_none() {
            v.standard = match (v.width, v.height, v.frame_rate) {
                (720, 480 | 486, Some(f)) if (f - 30.0 / 1.001).abs() < 0.1 => Some("NTSC"),
                (720, 576, Some(f)) if (f - 25.0).abs() < 0.1 => Some("PAL"),
                _ => None,
            }
            .map(str::to_string);
        }

        if v.display_aspect_ratio.is_none() && v.width > 0 && v.height > 0 {
            let par = v.sample_aspect_ratio.unwrap_or(1.0);
            v.display_aspect_ratio = Some((v.width as f64 * par) / v.height as f64);
        }

        report.videos.push(v);
    }

    fn build_audio_track(
        entry_data: &[u8],
        codec_box: &[u8; 4],
        codec_str: &str,
        ctx: &TrackCtx,
        report: &mut MediaReport,
    ) {
        let mut a = AudioTrack::default();
        a.stream_id = ctx.stream_id;
        a.codec_id = Some(codec_str.to_string());
        a.duration_ms = ctx.duration_ms;
        a.stream_size = ctx.stream_size;
        a.bit_rate = ctx.bit_rate;
        a.language = ctx.language.clone();
        a.default_flag = ctx.enabled;
        a.frame_count = (ctx.sample_count > 0).then_some(ctx.sample_count);

        // AudioSampleEntry fixed fields: channels at +24, sample size at +26, rate at +32.
        let mut entry_version = 0u16;
        if entry_data.len() >= 36 {
            entry_version = u16::from_be_bytes([entry_data[16], entry_data[17]]);
            let channels = u16::from_be_bytes([entry_data[24], entry_data[25]]) as u32;
            let sample_size = u16::from_be_bytes([entry_data[26], entry_data[27]]) as u8;
            let rate = u16::from_be_bytes([entry_data[32], entry_data[33]]) as u32;
            if channels > 0 {
                a.channels = channels;
                a.channel_layout = AudioChannelLayout::from_channel_count(channels);
            }
            if sample_size > 0 {
                a.bit_depth = Some(sample_size);
            }
            if rate > 0 {
                a.sampling_rate = rate;
            }
        }
        // A v2 entry carries the authoritative 64-bit float rate and channel count.
        if entry_version == 2 && entry_data.len() >= 52 {
            let rate = f64::from_be_bytes([
                entry_data[40],
                entry_data[41],
                entry_data[42],
                entry_data[43],
                entry_data[44],
                entry_data[45],
                entry_data[46],
                entry_data[47],
            ]);
            if rate > 0.0 {
                a.sampling_rate = rate as u32;
            }
            let channels = u32::from_be_bytes([
                entry_data[48],
                entry_data[49],
                entry_data[50],
                entry_data[51],
            ]);
            if channels > 0 {
                a.channels = channels;
                a.channel_layout = AudioChannelLayout::from_channel_count(channels);
            }
        }

        let (codec, info) = Self::audio_codec_from_fourcc(codec_box);
        a.format = codec.clone();
        a.format_info = info.map(str::to_string);
        a.compression_mode = Some(
            match codec {
                AudioCodec::PCM | AudioCodec::ALAC | AudioCodec::FLAC => "Lossless",
                _ => "Lossy",
            }
            .to_string(),
        );

        match codec {
            AudioCodec::AAC => {
                if let Some(esds) = Self::find_sample_entry_child(entry_data, b"esds") {
                    if let Some(asc) = Self::esds_decoder_config(esds) {
                        if let Ok(aac) = AacInfo::parse_audio_specific_config(asc) {
                            a.sampling_rate = aac.sampling_rate;
                            a.channels = aac.channels;
                            a.channel_layout = Some(aac.channel_layout);
                            a.format_profile = Some(aac.profile.to_string());
                            // MediaInfo spells this as fourcc-objectTypeIndication-audioObjectType.
                            if let Some(oti) = Self::esds_object_type(esds) {
                                a.codec_id =
                                    Some(format!("{codec_str}-{oti:X}-{}", aac.audio_object_type));
                            }
                        }
                    }
                    if let Some(rate) = Self::esds_avg_bitrate(esds) {
                        a.bit_rate = Some(rate);
                    }
                }
                // AAC has no meaningful container-level bit depth.
                a.bit_depth = None;
            }
            AudioCodec::ALAC => {
                if let Some(alac) = Self::find_sample_entry_child(entry_data, b"alac") {
                    // ALACSpecificConfig follows a 4-byte version/flags field.
                    let cfg = if alac.len() > 4 { &alac[4..] } else { alac };
                    if cfg.len() >= 24 {
                        a.bit_depth = Some(cfg[5]);
                        a.channels = cfg[9] as u32;
                        a.channel_layout = AudioChannelLayout::from_channel_count(a.channels);
                        a.sampling_rate = u32::from_be_bytes([cfg[20], cfg[21], cfg[22], cfg[23]]);
                    }
                }
            }
            AudioCodec::AC3 => {
                if let Some(dac3) = Self::find_sample_entry_child(entry_data, b"dac3") {
                    Self::apply_dac3(dac3, &mut a);
                }
                a.bit_depth = None;
            }
            AudioCodec::EAC3 => {
                if let Some(dec3) = Self::find_sample_entry_child(entry_data, b"dec3") {
                    Self::apply_dec3(dec3, &mut a);
                }
                a.bit_depth = None;
            }
            AudioCodec::Opus => {
                if let Some(dops) = Self::find_sample_entry_child(entry_data, b"dOps") {
                    if dops.len() >= 11 {
                        a.channels = dops[1] as u32;
                        a.channel_layout = AudioChannelLayout::from_channel_count(a.channels);
                        a.sampling_rate = u32::from_be_bytes([dops[4], dops[5], dops[6], dops[7]]);
                    }
                }
                // Opus always decodes to 48 kHz regardless of the original rate.
                if a.sampling_rate == 0 {
                    a.sampling_rate = 48000;
                }
                a.bit_depth = None;
            }
            AudioCodec::FLAC => {
                if let Some(dfla) = Self::find_sample_entry_child(entry_data, b"dfLa") {
                    // dfLa wraps raw FLAC metadata blocks after a version/flags field.
                    if dfla.len() > 4 {
                        if let Ok(info) = crate::audio::FlacStreamInfo::parse(&dfla[4..]) {
                            a.sampling_rate = info.sample_rate;
                            a.channels = info.channels;
                            a.channel_layout = AudioChannelLayout::from_channel_count(a.channels);
                            a.bit_depth = Some(info.bit_depth);
                        }
                    }
                }
            }
            AudioCodec::PCM => {
                // Sound sample entries store the sample size directly; endianness and
                // signedness come from the fourcc.
                a.format_profile = Some(Self::pcm_profile(codec_box).to_string());
            }
            _ => {}
        }

        if a.channels == 0 {
            a.channels = 2;
        }
        report.audios.push(a);
    }

    /// AVC decoder configuration record -> SPS.
    fn decode_avcc(entry_data: &[u8], v: &mut VideoTrack) {
        let Some(avcc) = Self::find_sample_entry_child(entry_data, b"avcC") else {
            return;
        };
        if avcc.len() < 8 {
            return;
        }
        let sps_len = u16::from_be_bytes([avcc[6], avcc[7]]) as usize;
        if avcc.len() < 8 + sps_len || sps_len == 0 {
            return;
        }
        if let Ok(sps) = AvcSps::parse(&avcc[8..8 + sps_len]) {
            if sps.width > 0 && sps.height > 0 {
                v.stored_width = Some(sps.width);
                v.stored_height = Some(sps.height);
                if v.width == 0 || v.height == 0 {
                    v.width = sps.width;
                    v.height = sps.height;
                }
            }
            v.format_profile = Some(sps.profile_name.to_string());
            v.format_level = Some(sps.level_name);
            v.bit_depth = sps.bit_depth;
            v.chroma_subsampling = Some(sps.chroma_subsampling);
            v.color_range = sps.color_range.or(v.color_range);
            v.color_primaries = sps.color_primaries;
            v.transfer_characteristics = sps.transfer_characteristics;
            v.matrix_coefficients = sps.matrix_coefficients;
            if let Some(fps) = sps.frame_rate {
                if v.frame_rate.is_none() {
                    v.frame_rate = Some(fps);
                }
            }
        }
    }

    fn decode_hvcc(entry_data: &[u8], v: &mut VideoTrack) {
        let Some(hvcc) = Self::find_sample_entry_child(entry_data, b"hvcC") else {
            return;
        };
        if hvcc.len() >= 23 {
            // General profile / chroma / bit depth are already in the config record.
            v.chroma_subsampling = Some(match hvcc[18] & 0x03 {
                0 => ChromaSubsampling::Monochrome,
                1 => ChromaSubsampling::YUV420,
                2 => ChromaSubsampling::YUV422,
                _ => ChromaSubsampling::YUV444,
            });
            let luma_depth = (hvcc[19] & 0x07) + 8;
            if (8..=16).contains(&luma_depth) {
                v.bit_depth = luma_depth;
            }

            let num_arrays = hvcc[22];
            let mut arr_off = 23;
            for _ in 0..num_arrays {
                if arr_off + 3 > hvcc.len() {
                    break;
                }
                let nal_type = hvcc[arr_off] & 0x3F;
                let num_nalus = u16::from_be_bytes([hvcc[arr_off + 1], hvcc[arr_off + 2]]) as usize;
                arr_off += 3;
                for _ in 0..num_nalus {
                    if arr_off + 2 > hvcc.len() {
                        break;
                    }
                    let nalu_len = u16::from_be_bytes([hvcc[arr_off], hvcc[arr_off + 1]]) as usize;
                    arr_off += 2;
                    if nal_type == 33 && arr_off + nalu_len <= hvcc.len() {
                        if let Ok(sps) = HevcSps::parse(&hvcc[arr_off..arr_off + nalu_len]) {
                            if sps.width > 0 && sps.height > 0 {
                                v.stored_width = Some(sps.width);
                                v.stored_height = Some(sps.height);
                                if v.width == 0 || v.height == 0 {
                                    v.width = sps.width;
                                    v.height = sps.height;
                                }
                            }
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
                        }
                    }
                    arr_off += nalu_len;
                }
            }
        }

        if let Some(dvcc) = Self::find_sample_entry_child(entry_data, b"dvcC")
            .or_else(|| Self::find_sample_entry_child(entry_data, b"dvvC"))
        {
            if dvcc.len() >= 4 {
                let profile = (dvcc[2] >> 1) & 0x7F;
                let level = ((dvcc[2] & 0x01) << 5) | ((dvcc[3] >> 3) & 0x1F);
                v.dolby_vision = Some(DolbyVisionInfo {
                    profile: DolbyVisionProfile::from_u8(profile),
                    level,
                    rpu_present: (dvcc[3] & 0x04) != 0,
                    el_present: (dvcc[3] & 0x02) != 0,
                    bl_present: (dvcc[3] & 0x01) != 0,
                    bl_signal_compatibility_id: Some(profile),
                    dm_version: Some("v2.9 / v4.0".to_string()),
                });
                v.hdr_format = Some("Dolby Vision / HDR10".to_string());
            }
        }
    }

    /// AV1CodecConfigurationRecord (av1C) - profile, bit depth and chroma live in 4 bytes.
    fn decode_av1c(entry_data: &[u8], v: &mut VideoTrack) {
        let Some(av1c) = Self::find_sample_entry_child(entry_data, b"av1C") else {
            return;
        };
        if av1c.len() < 4 {
            return;
        }
        let seq_profile = (av1c[1] >> 5) & 0x07;
        v.format_profile = Some(
            match seq_profile {
                0 => "Main",
                1 => "High",
                2 => "Professional",
                _ => "Unknown",
            }
            .to_string(),
        );
        let high_bitdepth = (av1c[2] & 0x40) != 0;
        let twelve_bit = (av1c[2] & 0x20) != 0;
        v.bit_depth = if twelve_bit {
            12
        } else if high_bitdepth {
            10
        } else {
            8
        };
        let mono = (av1c[2] & 0x10) != 0;
        let sub_x = (av1c[2] & 0x08) != 0;
        let sub_y = (av1c[2] & 0x04) != 0;
        v.chroma_subsampling = Some(if mono {
            ChromaSubsampling::Monochrome
        } else if sub_x && sub_y {
            ChromaSubsampling::YUV420
        } else if sub_x {
            ChromaSubsampling::YUV422
        } else {
            ChromaSubsampling::YUV444
        });
    }

    /// VPCodecConfigurationRecord (vpcC).
    fn decode_vpcc(entry_data: &[u8], v: &mut VideoTrack) {
        let Some(vpcc) = Self::find_sample_entry_child(entry_data, b"vpcC") else {
            return;
        };
        if vpcc.len() < 9 {
            return;
        }
        // Byte 0..3 is version/flags; the record starts at 4.
        let profile = vpcc[4];
        let bit_depth = (vpcc[6] >> 4) & 0x0F;
        let chroma = (vpcc[6] >> 1) & 0x07;
        let full_range = (vpcc[6] & 0x01) != 0;
        v.format_profile = Some(profile.to_string());
        if (8..=12).contains(&bit_depth) {
            v.bit_depth = bit_depth;
        }
        v.chroma_subsampling = Some(match chroma {
            0 | 1 => ChromaSubsampling::YUV420,
            2 => ChromaSubsampling::YUV422,
            3 => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV420,
        });
        v.color_range = Some(if full_range {
            ColorRange::Full
        } else {
            ColorRange::Limited
        });
        v.color_primaries = Some(ColorPrimaries::from_u8(vpcc[7]));
        v.transfer_characteristics = Some(TransferCharacteristics::from_u8(vpcc[8]));
        if vpcc.len() >= 10 {
            v.matrix_coefficients = Some(MatrixCoefficients::from_u8(vpcc[9]));
        }
    }

    /// ProRes: the fourcc fixes the profile, chroma and bit depth; the frame header in the
    /// first sample supplies the real colorimetry and scan type.
    fn decode_prores(
        entry_data: &[u8],
        codec_box: &[u8; 4],
        ctx: &TrackCtx,
        v: &mut VideoTrack,
        file: &[u8],
    ) {
        let variant = ProResVariant::from_fourcc(codec_box);
        v.format_profile = Some(variant.profile_name().to_string());
        v.format_commercial = Some(format!("Apple ProRes {}", variant.profile_name()));
        v.chroma_subsampling = Some(variant.chroma_subsampling());
        v.bit_depth = variant.bit_depth();
        v.compression_mode = Some("Lossy".to_string());
        // ProRes is intra-only with a variable per-frame size.
        v.bit_rate_mode = Some(BitrateMode::Variable);

        if let Some(range) = ctx.first_sample.clone() {
            if let Some(frame) = file.get(range) {
                if let Ok(header) = ProResHeader::parse(frame) {
                    if header.width > 0 && header.height > 0 {
                        v.stored_width = Some(header.width);
                        v.stored_height = Some(header.height);
                        if v.width == 0 || v.height == 0 {
                            v.width = header.width;
                            v.height = header.height;
                        }
                    }
                    // Chroma comes from the frame; bit depth stays profile-derived.
                    v.chroma_subsampling = Some(header.chroma_subsampling);
                    v.color_primaries = header.color_primaries;
                    v.transfer_characteristics = header.transfer_characteristics;
                    v.matrix_coefficients = header.matrix_coefficients;
                    v.format_version = Some(header.version.to_string());
                    v.scan_type = Some(header.scan_type().to_string());
                    v.scan_order = header.scan_order().map(str::to_string);
                    if let Some(fps) = header.frame_rate {
                        if v.frame_rate.is_none() {
                            v.frame_rate = Some(fps);
                        }
                    }
                    if let Some(dar) = header.display_aspect_ratio() {
                        if v.sample_aspect_ratio.is_none() {
                            v.display_aspect_ratio = Some(dar);
                        }
                    }
                    if let Some(bits) = header.alpha_bit_depth() {
                        v.extra
                            .insert("Alpha_Channel".to_string(), "Yes".to_string());
                        v.extra
                            .insert("Alpha_BitDepth".to_string(), bits.to_string());
                    }
                    if let Some(lib) = header.encoder_identifier() {
                        v.encoded_library = Some(lib);
                    }
                }
            }
        }

        // MediaInfo leaves the colour space unset for the 4444 family, whose frames may
        // carry an alpha plane alongside the three colour components.
        if variant.has_alpha() {
            v.color_space = None;
        }

        let _ = entry_data;
    }

    /// GoPro CineForm carries its frame geometry and sample format in the sample header
    /// rather than in the sample entry.
    fn decode_cineform(ctx: &TrackCtx, v: &mut VideoTrack, file: &[u8]) {
        // CineForm codes 10-bit 4:2:2 unless the sample header says otherwise.
        v.bit_depth = 10;
        v.chroma_subsampling = Some(ChromaSubsampling::YUV422);
        v.compression_mode = Some("Lossy".to_string());

        let Some(range) = ctx.first_sample.clone() else {
            return;
        };
        let Some(sample) = file.get(range) else {
            return;
        };
        if let Ok(header) = CineFormHeader::parse(sample) {
            if header.width > 0 && header.height > 0 {
                v.stored_width = Some(header.width);
                v.stored_height = Some(header.height);
                if v.width == 0 || v.height == 0 {
                    v.width = header.width;
                    v.height = header.height;
                }
            }
            if (8..=16).contains(&header.bit_depth) {
                v.bit_depth = header.bit_depth;
            }
            v.chroma_subsampling = Some(header.chroma_subsampling);
        }
    }

    fn apply_colr(colr: &[u8], v: &mut VideoTrack) {
        if colr.len() < 4 {
            return;
        }
        let kind = &colr[0..4];
        if (kind == b"nclx" || kind == b"nclc") && colr.len() >= 10 {
            v.color_primaries = Some(ColorPrimaries::from_u8(
                u16::from_be_bytes([colr[4], colr[5]]) as u8,
            ));
            v.transfer_characteristics = Some(TransferCharacteristics::from_u8(
                u16::from_be_bytes([colr[6], colr[7]]) as u8,
            ));
            v.matrix_coefficients =
                Some(MatrixCoefficients::from_u8(
                    u16::from_be_bytes([colr[8], colr[9]]) as u8,
                ));
            if kind == b"nclx" && colr.len() >= 11 {
                v.color_range = Some(if colr[10] & 0x80 != 0 {
                    ColorRange::Full
                } else {
                    ColorRange::Limited
                });
            }
        }
    }

    fn apply_dac3(dac3: &[u8], a: &mut AudioTrack) {
        if dac3.len() < 3 {
            return;
        }
        // Bit-packed as fscod(2) bsid(5) bsmod(3) acmod(3) lfeon(1) bit_rate_code(5),
        // so acmod and bit_rate_code straddle byte boundaries.
        let fscod = (dac3[0] >> 6) & 0x03;
        let acmod = ((dac3[1] >> 3) & 0x07) as usize;
        let lfeon = (dac3[1] >> 2) & 0x01;
        let bit_rate_code = (((dac3[1] & 0x03) << 3) | ((dac3[2] >> 5) & 0x07)) as usize;

        const RATES: [u32; 3] = [48000, 44100, 32000];
        if (fscod as usize) < RATES.len() {
            a.sampling_rate = RATES[fscod as usize];
        }
        const CHANNELS: [u32; 8] = [2, 1, 2, 3, 3, 4, 4, 5];
        let mut channels = CHANNELS[acmod];
        if lfeon == 1 {
            channels += 1;
        }
        a.channels = channels;
        a.channel_layout = AudioChannelLayout::from_channel_count(channels);

        const BITRATES: [u64; 19] = [
            32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 576, 640,
        ];
        if bit_rate_code < BITRATES.len() {
            a.bit_rate = Some(BITRATES[bit_rate_code] * 1000);
            a.bit_rate_mode = Some(BitrateMode::Constant);
        }
    }

    fn apply_dec3(dec3: &[u8], a: &mut AudioTrack) {
        if dec3.len() < 5 {
            return;
        }
        let data_rate = (((dec3[0] as u64) & 0x1F) << 8) | (dec3[1] as u64);
        if data_rate > 0 {
            a.bit_rate = Some(data_rate * 1000);
        }
        // First independent substream descriptor starts at byte 2.
        let fscod = (dec3[2] >> 6) & 0x03;
        let acmod = ((dec3[3] >> 1) & 0x07) as usize;
        let lfeon = dec3[3] & 0x01;
        const RATES: [u32; 3] = [48000, 44100, 32000];
        if (fscod as usize) < RATES.len() {
            a.sampling_rate = RATES[fscod as usize];
        }
        const CHANNELS: [u32; 8] = [2, 1, 2, 3, 3, 4, 4, 5];
        let mut channels = CHANNELS[acmod];
        if lfeon == 1 {
            channels += 1;
        }
        a.channels = channels;
        a.channel_layout = AudioChannelLayout::from_channel_count(channels);
    }

    /// Walks an esds descriptor tree and returns the DecoderSpecificInfo (tag 0x05) payload.
    fn esds_decoder_config(esds: &[u8]) -> Option<&[u8]> {
        Self::esds_find_tag(esds, 0x05)
    }

    /// The DecoderConfigDescriptor's objectTypeIndication (0x40 for MPEG-4 audio).
    fn esds_object_type(esds: &[u8]) -> Option<u8> {
        Self::esds_find_tag(esds, 0x04)?.first().copied()
    }

    fn esds_avg_bitrate(esds: &[u8]) -> Option<u64> {
        // DecoderConfigDescriptor (0x04): objectType(1) streamType(1) bufferSize(3)
        // maxBitrate(4) avgBitrate(4)
        let dcd = Self::esds_find_tag(esds, 0x04)?;
        if dcd.len() < 13 {
            return None;
        }
        let avg = u32::from_be_bytes([dcd[9], dcd[10], dcd[11], dcd[12]]) as u64;
        (avg > 0).then_some(avg)
    }

    fn esds_find_tag(data: &[u8], want: u8) -> Option<&[u8]> {
        // Skip the 4-byte version/flags prefix of the esds full box.
        let mut pos = if data.len() > 4 { 4 } else { 0 };
        Self::descriptor_scan(&data[pos..], want, 0).or_else(|| {
            pos = 0;
            Self::descriptor_scan(&data[pos..], want, 0)
        })
    }

    fn descriptor_scan(data: &[u8], want: u8, depth: u8) -> Option<&[u8]> {
        if depth > 4 {
            return None;
        }
        let mut pos = 0usize;
        while pos + 2 <= data.len() {
            let tag = data[pos];
            pos += 1;
            // Length uses up to four 7-bit continuation bytes.
            let mut len = 0usize;
            for _ in 0..4 {
                if pos >= data.len() {
                    return None;
                }
                let b = data[pos];
                pos += 1;
                len = (len << 7) | (b & 0x7F) as usize;
                if b & 0x80 == 0 {
                    break;
                }
            }
            let end = (pos + len).min(data.len());
            let body = &data[pos..end];
            if tag == want {
                return Some(body);
            }
            // ES_Descriptor (0x03) and DecoderConfigDescriptor (0x04) nest children.
            let nested = match tag {
                0x03 if body.len() > 3 => {
                    // ES_ID(2) + flags(1); optional fields are rare in MP4 and skipped.
                    Self::descriptor_scan(&body[3..], want, depth + 1)
                }
                0x04 if body.len() > 13 => Self::descriptor_scan(&body[13..], want, depth + 1),
                _ => None,
            };
            if nested.is_some() {
                return nested;
            }
            if len == 0 {
                break;
            }
            pos = end;
        }
        None
    }

    fn pcm_profile(fourcc: &[u8; 4]) -> &'static str {
        match fourcc {
            b"twos" => "Big / Signed",
            b"sowt" => "Little / Signed",
            b"raw " => "Little / Unsigned",
            b"in24" | b"in32" => "Big / Signed",
            b"fl32" | b"fl64" => "Big / Float",
            _ => "Signed",
        }
    }

    fn video_codec_from_fourcc(fourcc: &[u8; 4]) -> (VideoCodec, Option<&'static str>) {
        match fourcc {
            b"avc1" | b"avc2" | b"avc3" | b"avc4" => {
                (VideoCodec::AVC, Some("Advanced Video Coding"))
            }
            b"hvc1" | b"hev1" | b"hvc2" | b"hev2" | b"dvh1" | b"dvhe" => {
                (VideoCodec::HEVC, Some("High Efficiency Video Coding"))
            }
            b"vvc1" | b"vvi1" => (VideoCodec::VVC, Some("Versatile Video Coding")),
            b"av01" => (VideoCodec::AV1, Some("AOMedia Video 1")),
            b"vp09" => (VideoCodec::VP9, Some("Google VP9")),
            b"vp08" => (VideoCodec::VP8, Some("Google VP8")),
            b"apco" | b"apcs" | b"apcn" | b"apch" | b"ap4h" | b"ap4x" | b"aprh" | b"aprn" => {
                (VideoCodec::ProRes, Some("Apple ProRes"))
            }
            b"mp4v" => (VideoCodec::MPEG4Visual, Some("MPEG-4 Visual")),
            b"mpeg" | b"mp2v" | b"m2v1" | b"hdv1" | b"hdv2" | b"hdv3" => {
                (VideoCodec::MPEG2Video, Some("MPEG-2 Video"))
            }
            b"mp1v" => (VideoCodec::MPEG1Video, Some("MPEG-1 Video")),
            b"vc-1" | b"WVC1" => (VideoCodec::VC1, Some("SMPTE 421M")),
            b"cfhd" | b"CFHD" => (VideoCodec::CineForm, Some("GoPro CineForm")),
            b"AVdn" | b"AVdh" => (VideoCodec::DNxHD, Some("Avid DNxHD / DNxHR")),
            b"FFV1" => (VideoCodec::FFV1, Some("FFmpeg Video 1")),
            b"dvc " | b"dvcp" | b"dvpp" | b"dv5n" | b"dv5p" | b"dvh5" | b"dvh6" | b"dvhq" => {
                (VideoCodec::DV, Some("Digital Video"))
            }
            b"raw " | b"2vuy" | b"yuv2" | b"v210" | b"v410" | b"r210" => {
                (VideoCodec::Raw, Some("Uncompressed"))
            }
            b"jpeg" | b"mjpa" | b"mjpb" => {
                (VideoCodec::Other("JPEG".to_string()), Some("Motion JPEG"))
            }
            b"png " => (VideoCodec::Other("PNG".to_string()), None),
            _ => (
                VideoCodec::Other(String::from_utf8_lossy(fourcc).trim().to_string()),
                None,
            ),
        }
    }

    fn audio_codec_from_fourcc(fourcc: &[u8; 4]) -> (AudioCodec, Option<&'static str>) {
        match fourcc {
            b"mp4a" => (AudioCodec::AAC, Some("Advanced Audio Coding")),
            b"ac-3" | b"sac3" => (AudioCodec::AC3, Some("Dolby Digital")),
            b"ec-3" => (AudioCodec::EAC3, Some("Dolby Digital Plus")),
            b"ac-4" => (AudioCodec::AC4, Some("Dolby AC-4")),
            b"mlpa" => (AudioCodec::TrueHD, Some("Dolby TrueHD")),
            b"alac" => (AudioCodec::ALAC, Some("Apple Lossless")),
            b"fLaC" => (AudioCodec::FLAC, Some("Free Lossless Audio Codec")),
            b"Opus" => (AudioCodec::Opus, Some("Opus")),
            b"dtsc" | b"dtse" => (AudioCodec::DTS, Some("DTS")),
            b"dtsh" | b"dtsl" => (AudioCodec::DTSHD, Some("DTS-HD")),
            b"mha1" | b"mha2" | b"mhm1" | b"mhm2" => (AudioCodec::MPEGH, Some("MPEG-H 3D Audio")),
            b"samr" => (AudioCodec::AMR_NB, Some("AMR Narrowband")),
            b"sawb" => (AudioCodec::AMR_WB, Some("AMR Wideband")),
            b".mp3" | b"ms\x00\x55" => (AudioCodec::MPEGAudioLayer3, Some("MPEG Audio Layer 3")),
            b"twos" | b"sowt" | b"raw " | b"in24" | b"in32" | b"fl32" | b"fl64" | b"lpcm"
            | b"ipcm" | b"NONE" => (AudioCodec::PCM, Some("Pulse Code Modulation")),
            b"agsm" => (AudioCodec::Other("GSM".to_string()), None),
            b"ima4" => (AudioCodec::Other("ADPCM".to_string()), Some("IMA ADPCM")),
            _ => (
                AudioCodec::Other(String::from_utf8_lossy(fourcc).trim().to_string()),
                None,
            ),
        }
    }

    fn parse_udta(data: &[u8], report: &mut MediaReport) {
        if let Some(meta) = Self::find_child_box(data, b"meta") {
            let meta_payload = if meta.len() >= 4 { &meta[4..] } else { meta };
            if let Some(ilst) = Self::find_child_box(meta_payload, b"ilst") {
                if let Ok(tags) = ItunesTags::parse(ilst) {
                    report.general.title = tags.title;
                    report.general.artist = tags.artist;
                    report.general.album = tags.album;
                    report.general.recorded_date = tags.date;
                    report.general.genre = tags.genre;
                    report.general.encoded_application = tags.encoder;
                    if tags.cover_data.is_some() {
                        report.general.cover_art_present = true;
                        report.general.cover_mime = tags.cover_mime;
                    }
                }
            }
        }
    }

    /// Finds a child box inside a sample entry.
    ///
    /// A sample entry is a box whose payload begins with a fixed-size record before any
    /// child boxes, so scanning it as a plain box list (as `find_child_box` does) walks
    /// straight past the entry and finds nothing.
    fn find_sample_entry_child<'a>(entry: &'a [u8], target: &[u8; 4]) -> Option<&'a [u8]> {
        let header_len = Self::sample_entry_header_len(entry)?;
        if header_len >= entry.len() {
            return None;
        }
        let children = &entry[header_len..];
        Self::find_child_box(children, target).or_else(|| {
            // QuickTime sound descriptions wrap their extensions in a `wave` atom.
            let wave = Self::find_child_box(children, b"wave")?;
            Self::find_child_box(wave, target)
        })
    }

    /// Size of the fixed record at the start of a sample entry, including the 8-byte box
    /// header and the 8 bytes of `reserved` + `data_reference_index` common to all entries.
    fn sample_entry_header_len(entry: &[u8]) -> Option<usize> {
        if entry.len() < 16 {
            return None;
        }
        let fourcc: [u8; 4] = entry[4..8].try_into().ok()?;
        if Self::is_audio_sample_entry(&fourcc) {
            // QuickTime sound description versions add trailing fields to the fixed part.
            let version = u16::from_be_bytes([entry[16], entry[17]]);
            Some(match version {
                1 => 52,
                2 => 72,
                _ => 36,
            })
        } else {
            // VisualSampleEntry: 16 common + 70 bytes through `pre_defined`.
            Some(86)
        }
    }

    fn is_audio_sample_entry(fourcc: &[u8; 4]) -> bool {
        matches!(
            fourcc,
            b"mp4a"
                | b"ac-3"
                | b"sac3"
                | b"ec-3"
                | b"ac-4"
                | b"mlpa"
                | b"alac"
                | b"fLaC"
                | b"Opus"
                | b"dtsc"
                | b"dtse"
                | b"dtsh"
                | b"dtsl"
                | b"mha1"
                | b"mha2"
                | b"mhm1"
                | b"mhm2"
                | b"samr"
                | b"sawb"
                | b".mp3"
                | b"twos"
                | b"sowt"
                | b"raw "
                | b"in24"
                | b"in32"
                | b"fl32"
                | b"fl64"
                | b"lpcm"
                | b"ipcm"
                | b"NONE"
                | b"agsm"
                | b"ima4"
        )
    }

    fn find_child_box<'a>(data: &'a [u8], target_type: &[u8; 4]) -> Option<&'a [u8]> {
        let mut offset = 0;
        while offset + 8 <= data.len() {
            if let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) {
                if &box_type == target_type {
                    let payload_offset = offset + header_len;
                    let payload_size = box_size.saturating_sub(header_len);
                    let payload = if payload_offset + payload_size <= data.len() {
                        &data[payload_offset..payload_offset + payload_size]
                    } else {
                        &data[payload_offset..]
                    };
                    return Some(payload);
                }
                offset += box_size;
                if box_size == 0 {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }
}
