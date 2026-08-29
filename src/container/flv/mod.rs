use crate::audio::AacInfo;
use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use crate::video::AvcSps;

/// Flash Video (FLV) container demuxer.
pub struct FlvDemuxer;

/// A decoded AMF0 value from an FLV script tag.
enum Amf0 {
    Number(f64),
    Boolean(bool),
    Str(String),
    Object(Vec<(String, Amf0)>),
    Null,
    Unsupported,
}

impl Amf0 {
    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Boolean(b) => Some(*b as u8 as f64),
            _ => None,
        }
    }
}

impl FlvDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 9 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 9,
                actual: data.len(),
            });
        }

        if !data.starts_with(b"FLV") {
            return Err(MediaInfoError::InvalidData(
                "Not a valid FLV file".to_string(),
            ));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::FLV;
        report.general.file_size = data.len() as u64;

        let flags = data[4];
        let has_audio = (flags & 0x04) != 0;
        let has_video = (flags & 0x01) != 0;
        let data_offset = u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as usize;

        let mut video = has_video.then(|| {
            let mut v = VideoTrack::default();
            v.stream_id = 1;
            v
        });
        let mut audio = has_audio.then(|| {
            let mut a = AudioTrack::default();
            a.stream_id = if has_video { 2 } else { 1 };
            a
        });

        let mut metadata = Vec::new();
        let mut video_bytes = 0u64;
        let mut audio_bytes = 0u64;
        let mut last_timestamp_ms = 0u32;

        // Walk the tag list. Codec configuration arrives in the first tag of each stream,
        // so the walk mainly accumulates payload sizes after that.
        let mut offset = data_offset.max(9) + 4; // skip the header and PreviousTagSize0
        while offset + 11 <= data.len() {
            let tag_type = data[offset] & 0x1F;
            let data_size =
                u32::from_be_bytes([0, data[offset + 1], data[offset + 2], data[offset + 3]])
                    as usize;
            let timestamp = u32::from_be_bytes([
                data[offset + 7],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
            ]);
            let body_off = offset + 11;
            if body_off + data_size > data.len() {
                break;
            }
            let body = &data[body_off..body_off + data_size];
            last_timestamp_ms = last_timestamp_ms.max(timestamp);

            match tag_type {
                9 => {
                    // The leading codec framing bytes and the decoder configuration tag
                    // are container overhead, not part of the coded video.
                    video_bytes += Self::video_frame_len(body) as u64;
                    if let Some(v) = video.as_mut() {
                        Self::parse_video_tag(body, v);
                    }
                }
                8 => {
                    audio_bytes += Self::audio_frame_len(body) as u64;
                    if let Some(a) = audio.as_mut() {
                        Self::parse_audio_tag(body, a);
                    }
                }
                18 => {
                    metadata = Self::parse_script_tag(body);
                }
                _ => {}
            }

            offset = body_off + data_size + 4; // PreviousTagSize
        }

        let duration_ms = Self::metadata_number(&metadata, "duration")
            .map(|d| d * 1000.0)
            .filter(|d| *d > 0.0)
            .unwrap_or(last_timestamp_ms as f64);
        if duration_ms > 0.0 {
            report.general.duration_ms = Some(duration_ms);
            report.general.overall_bitrate =
                Some(((data.len() * 8) as f64 / (duration_ms / 1000.0)) as u64);
        }

        if let Some(mut v) = video {
            if v.width == 0 {
                v.width = Self::metadata_number(&metadata, "width").unwrap_or(0.0) as u32;
            }
            if v.height == 0 {
                v.height = Self::metadata_number(&metadata, "height").unwrap_or(0.0) as u32;
            }
            if v.frame_rate.is_none() {
                v.frame_rate = Self::metadata_number(&metadata, "framerate")
                    .or_else(|| Self::metadata_number(&metadata, "videoframerate"))
                    .filter(|f| *f > 0.0);
            }
            v.duration_ms = (duration_ms > 0.0).then_some(duration_ms);
            v.stream_size = Some(video_bytes);
            v.bit_rate = if duration_ms > 0.0 {
                Some((video_bytes as f64 * 8.0 / (duration_ms / 1000.0)) as u64)
            } else {
                Self::metadata_number(&metadata, "videodatarate").map(|r| (r * 1000.0) as u64)
            };
            if v.color_space.is_none() {
                v.color_space = Some("YUV".to_string());
            }
            if v.width > 0 && v.height > 0 {
                v.display_aspect_ratio = Some(v.width as f64 / v.height as f64);
            }
            report.videos.push(v);
        }

        if let Some(mut a) = audio {
            a.duration_ms = (duration_ms > 0.0).then_some(duration_ms);
            a.stream_size = Some(audio_bytes);
            a.bit_rate = if duration_ms > 0.0 {
                Some((audio_bytes as f64 * 8.0 / (duration_ms / 1000.0)) as u64)
            } else {
                Self::metadata_number(&metadata, "audiodatarate").map(|r| (r * 1000.0) as u64)
            };
            report.audios.push(a);
        }

        if let Some(Amf0::Str(enc)) = Self::metadata_get(&metadata, "encoder") {
            report.general.encoded_application = Some(enc.clone());
        }

        Ok(report)
    }

    /// Coded video bytes in a tag, excluding the codec framing prefix. Sequence header
    /// tags carry decoder configuration rather than picture data and count as zero.
    fn video_frame_len(body: &[u8]) -> usize {
        if body.is_empty() {
            return 0;
        }
        let codec_id = body[0] & 0x0F;
        // AVC and HEVC prefix each tag with a packet type and a composition time offset.
        let prefix = if matches!(codec_id, 7 | 12) { 5 } else { 1 };
        if matches!(codec_id, 7 | 12) && body.get(1) == Some(&0) {
            return 0;
        }
        body.len().saturating_sub(prefix)
    }

    /// Coded audio bytes in a tag, excluding the codec framing prefix.
    fn audio_frame_len(body: &[u8]) -> usize {
        if body.is_empty() {
            return 0;
        }
        let sound_format = (body[0] >> 4) & 0x0F;
        let prefix = if sound_format == 10 { 2 } else { 1 };
        if sound_format == 10 && body.get(1) == Some(&0) {
            return 0;
        }
        body.len().saturating_sub(prefix)
    }

    /// Video tag: a 4-bit frame type and 4-bit codec id, then codec-specific data.
    fn parse_video_tag(body: &[u8], v: &mut VideoTrack) {
        if body.is_empty() {
            return;
        }
        let codec_id = body[0] & 0x0F;
        if v.codec_id.is_none() {
            v.codec_id = Some(codec_id.to_string());
            let (codec, info) = match codec_id {
                2 => (
                    VideoCodec::Other("H.263".to_string()),
                    Some("Sorenson H.263"),
                ),
                3 => (
                    VideoCodec::Other("Screen video".to_string()),
                    Some("Screen video"),
                ),
                4 | 5 => (VideoCodec::Other("VP6".to_string()), Some("On2 VP6")),
                6 => (
                    VideoCodec::Other("Screen video 2".to_string()),
                    Some("Screen video version 2"),
                ),
                7 => (VideoCodec::AVC, Some("Advanced Video Coding")),
                12 => (VideoCodec::HEVC, Some("High Efficiency Video Coding")),
                _ => (VideoCodec::Other(format!("FLV codec {codec_id}")), None),
            };
            v.format = codec;
            v.format_info = info.map(str::to_string);
        }

        // AVC/HEVC packet type 0 carries the decoder configuration record.
        if (codec_id == 7 || codec_id == 12) && body.len() > 5 && body[1] == 0 {
            Self::apply_avcc(&body[5..], v);
        }
    }

    fn apply_avcc(avcc: &[u8], v: &mut VideoTrack) {
        if avcc.len() < 8 {
            return;
        }
        let sps_len = u16::from_be_bytes([avcc[6], avcc[7]]) as usize;
        if sps_len == 0 || avcc.len() < 8 + sps_len {
            return;
        }
        if let Ok(sps) = AvcSps::parse(&avcc[8..8 + sps_len]) {
            v.width = sps.width;
            v.height = sps.height;
            v.stored_width = Some(sps.width);
            v.stored_height = Some(sps.height);
            v.format_profile = Some(sps.profile_name.to_string());
            v.format_level = Some(sps.level_name);
            v.bit_depth = sps.bit_depth;
            v.chroma_subsampling = Some(sps.chroma_subsampling);
            v.color_range = sps.color_range.or(v.color_range);
            v.color_primaries = sps.color_primaries;
            v.transfer_characteristics = sps.transfer_characteristics;
            v.matrix_coefficients = sps.matrix_coefficients;
            if v.frame_rate.is_none() {
                v.frame_rate = sps.frame_rate;
            }
            v.scan_type = Some(
                if sps.progressive {
                    "Progressive"
                } else {
                    "Interlaced"
                }
                .to_string(),
            );
        }
    }

    /// Audio tag: a packed byte of format, rate, sample size and channel count.
    fn parse_audio_tag(body: &[u8], a: &mut AudioTrack) {
        if body.is_empty() {
            return;
        }
        let sound_format = (body[0] >> 4) & 0x0F;
        let sound_rate = (body[0] >> 2) & 0x03;
        let sound_size = (body[0] >> 1) & 0x01;
        let sound_type = body[0] & 0x01;

        if a.codec_id.is_none() {
            a.codec_id = Some(sound_format.to_string());
            let (codec, info) = match sound_format {
                0 => (AudioCodec::PCM, Some("Linear PCM, platform endian")),
                1 => (AudioCodec::Other("ADPCM".to_string()), Some("ADPCM")),
                2 | 14 => (AudioCodec::MPEGAudioLayer3, Some("MPEG Audio Layer 3")),
                3 => (AudioCodec::PCM, Some("Linear PCM, little endian")),
                4..=6 => (
                    AudioCodec::Other("Nellymoser".to_string()),
                    Some("Nellymoser"),
                ),
                7 => (AudioCodec::Other("G.711".to_string()), Some("G.711 A-law")),
                8 => (AudioCodec::Other("G.711".to_string()), Some("G.711 mu-law")),
                10 => (AudioCodec::AAC, Some("Advanced Audio Coding")),
                11 => (AudioCodec::Other("Speex".to_string()), Some("Speex")),
                _ => (AudioCodec::Other(format!("FLV codec {sound_format}")), None),
            };
            a.format = codec;
            a.format_info = info.map(str::to_string);

            // AAC in FLV is always signalled as 44 kHz stereo regardless of the real
            // configuration, so only trust these fields for the other codecs.
            if sound_format != 10 {
                a.sampling_rate = match sound_rate {
                    0 => 5512,
                    1 => 11025,
                    2 => 22050,
                    _ => 44100,
                };
                a.channels = if sound_type == 1 { 2 } else { 1 };
                a.channel_layout = AudioChannelLayout::from_channel_count(a.channels);
                a.bit_depth = Some(if sound_size == 1 { 16 } else { 8 });
            }
        }

        // AAC packet type 0 carries the AudioSpecificConfig.
        if sound_format == 10 && body.len() > 2 && body[1] == 0 {
            if let Ok(aac) = AacInfo::parse_audio_specific_config(&body[2..]) {
                a.sampling_rate = aac.sampling_rate;
                a.channels = aac.channels;
                a.channel_layout = Some(aac.channel_layout);
                a.format_profile = Some(aac.profile.to_string());
                a.codec_id = Some(format!("{sound_format}-{}", aac.audio_object_type));
            }
        }
    }

    /// Script tag: an AMF0 method name followed by its argument, normally `onMetaData`.
    fn parse_script_tag(body: &[u8]) -> Vec<(String, Amf0)> {
        let mut pos = 0;
        let Some(Amf0::Str(_name)) = Self::read_amf0(body, &mut pos, 0) else {
            return Vec::new();
        };
        match Self::read_amf0(body, &mut pos, 0) {
            Some(Amf0::Object(props)) => props,
            _ => Vec::new(),
        }
    }

    fn metadata_get<'a>(meta: &'a [(String, Amf0)], key: &str) -> Option<&'a Amf0> {
        meta.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    fn metadata_number(meta: &[(String, Amf0)], key: &str) -> Option<f64> {
        Self::metadata_get(meta, key).and_then(Amf0::as_f64)
    }

    fn read_amf0(data: &[u8], pos: &mut usize, depth: u8) -> Option<Amf0> {
        if depth > 8 || *pos >= data.len() {
            return None;
        }
        let marker = data[*pos];
        *pos += 1;
        match marker {
            0x00 => {
                let bytes: [u8; 8] = data.get(*pos..*pos + 8)?.try_into().ok()?;
                *pos += 8;
                Some(Amf0::Number(f64::from_be_bytes(bytes)))
            }
            0x01 => {
                let b = *data.get(*pos)?;
                *pos += 1;
                Some(Amf0::Boolean(b != 0))
            }
            0x02 => Self::read_amf0_string(data, pos).map(Amf0::Str),
            0x03 => Self::read_amf0_properties(data, pos, depth).map(Amf0::Object),
            0x05 | 0x06 => Some(Amf0::Null),
            0x08 => {
                // ECMA array: a count hint followed by the same layout as an object.
                *pos += 4;
                Self::read_amf0_properties(data, pos, depth).map(Amf0::Object)
            }
            0x0A => {
                let count = u32::from_be_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?) as usize;
                *pos += 4;
                for _ in 0..count.min(1024) {
                    Self::read_amf0(data, pos, depth + 1)?;
                }
                Some(Amf0::Unsupported)
            }
            0x0B => {
                *pos += 10; // date: f64 timestamp + int16 timezone
                Some(Amf0::Unsupported)
            }
            _ => None,
        }
    }

    fn read_amf0_properties(
        data: &[u8],
        pos: &mut usize,
        depth: u8,
    ) -> Option<Vec<(String, Amf0)>> {
        let mut props = Vec::new();
        loop {
            let key = Self::read_amf0_string(data, pos)?;
            if key.is_empty() {
                // An empty key introduces the object-end marker.
                if data.get(*pos) == Some(&0x09) {
                    *pos += 1;
                }
                return Some(props);
            }
            let value = Self::read_amf0(data, pos, depth + 1)?;
            props.push((key, value));
            if props.len() > 512 {
                return Some(props);
            }
        }
    }

    fn read_amf0_string(data: &[u8], pos: &mut usize) -> Option<String> {
        let len = u16::from_be_bytes(data.get(*pos..*pos + 2)?.try_into().ok()?) as usize;
        *pos += 2;
        let bytes = data.get(*pos..*pos + len)?;
        *pos += len;
        Some(String::from_utf8_lossy(bytes).to_string())
    }
}
