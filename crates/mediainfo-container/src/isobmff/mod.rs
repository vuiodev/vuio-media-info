use mediainfo_audio::AacInfo;
use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::ItunesTags;
use mediainfo_video::{AvcSps, HevcSps};

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
                    Self::parse_moov(payload, &mut report);
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
        report.general.format_profile = Some(compat_brands.join(" "));
        report.general.format_version = Some(minor_version.to_string());
    }

    fn parse_moov(data: &[u8], report: &mut MediaReport) {
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
                        Self::parse_trak(payload, track_id_counter, report);
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

    fn parse_trak(data: &[u8], track_id: u32, report: &mut MediaReport) {
        let mut offset = 0;
        let mut tkhd_width = 0u32;
        let mut tkhd_height = 0u32;

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
                    b"tkhd" => {
                        if payload.len() >= 84 {
                            let version = payload[0];
                            let w_offset = if version == 1 { 88 } else { 76 };
                            let h_offset = if version == 1 { 92 } else { 80 };
                            if payload.len() >= h_offset + 4 {
                                tkhd_width =
                                    u16::from_be_bytes([payload[w_offset], payload[w_offset + 1]])
                                        as u32;
                                tkhd_height =
                                    u16::from_be_bytes([payload[h_offset], payload[h_offset + 1]])
                                        as u32;
                            }
                        }
                    }
                    b"mdia" => {
                        Self::parse_mdia(payload, track_id, tkhd_width, tkhd_height, report);
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

    fn parse_mdia(
        data: &[u8],
        track_id: u32,
        tkhd_width: u32,
        tkhd_height: u32,
        report: &mut MediaReport,
    ) {
        let mut offset = 0;
        let mut handler = [0u8; 4];

        while offset + 8 <= data.len() {
            if let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) {
                let payload_offset = offset + header_len;
                let payload_size = box_size.saturating_sub(header_len);
                let payload = if payload_offset + payload_size <= data.len() {
                    &data[payload_offset..payload_offset + payload_size]
                } else {
                    &data[payload_offset..]
                };

                if &box_type == b"hdlr" && payload.len() >= 12 {
                    handler = [payload[8], payload[9], payload[10], payload[11]];
                } else if &box_type == b"minf" {
                    Self::parse_minf(payload, track_id, &handler, tkhd_width, tkhd_height, report);
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

    fn parse_minf(
        data: &[u8],
        track_id: u32,
        handler: &[u8; 4],
        tkhd_width: u32,
        tkhd_height: u32,
        report: &mut MediaReport,
    ) {
        let mut offset = 0;
        while offset + 8 <= data.len() {
            if let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) {
                let payload_offset = offset + header_len;
                let payload_size = box_size.saturating_sub(header_len);
                let payload = if payload_offset + payload_size <= data.len() {
                    &data[payload_offset..payload_offset + payload_size]
                } else {
                    &data[payload_offset..]
                };

                if &box_type == b"stbl" {
                    Self::parse_stbl(payload, track_id, handler, tkhd_width, tkhd_height, report);
                    break;
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

    fn parse_stbl(
        data: &[u8],
        track_id: u32,
        handler: &[u8; 4],
        tkhd_width: u32,
        tkhd_height: u32,
        report: &mut MediaReport,
    ) {
        let mut offset = 0;
        while offset + 8 <= data.len() {
            if let Ok((box_type, box_size, header_len)) = Self::read_box_header(data, offset) {
                let payload_offset = offset + header_len;
                let payload_size = box_size.saturating_sub(header_len);
                let payload = if payload_offset + payload_size <= data.len() {
                    &data[payload_offset..payload_offset + payload_size]
                } else {
                    &data[payload_offset..]
                };

                if &box_type == b"stsd" && payload.len() >= 8 {
                    let entry_data = &payload[8..];
                    if entry_data.len() >= 8 {
                        let (codec_box, _, _) =
                            Self::read_box_header(entry_data, 0).unwrap_or(([0; 4], 0, 0));
                        let codec_str = String::from_utf8_lossy(&codec_box).to_string();

                        if handler == b"vide" {
                            let mut v = VideoTrack::default();
                            v.stream_id = track_id;
                            v.codec_id = Some(codec_str.clone());
                            v.width = tkhd_width;
                            v.height = tkhd_height;

                            if codec_box == *b"avc1" || codec_box == *b"avc3" {
                                v.format = VideoCodec::AVC;
                                v.format_info = Some("Advanced Video Coding".to_string());
                                if let Some(avcc) = Self::find_child_box(entry_data, b"avcC") {
                                    if avcc.len() >= 7 {
                                        let sps_len =
                                            u16::from_be_bytes([avcc[6], avcc[7]]) as usize;
                                        if avcc.len() >= 8 + sps_len {
                                            let sps_bytes = &avcc[8..8 + sps_len];
                                            if let Ok(sps) = AvcSps::parse(sps_bytes) {
                                                v.width = sps.width;
                                                v.height = sps.height;
                                                v.format_profile =
                                                    Some(sps.profile_name.to_string());
                                                v.format_level = Some(sps.level_name);
                                                v.bit_depth = sps.bit_depth;
                                                v.chroma_subsampling = Some(sps.chroma_subsampling);
                                                v.color_range = sps.color_range;
                                                v.color_primaries = sps.color_primaries;
                                                v.transfer_characteristics =
                                                    sps.transfer_characteristics;
                                                v.matrix_coefficients = sps.matrix_coefficients;
                                            }
                                        }
                                    }
                                }
                            } else if codec_box == *b"hvc1"
                                || codec_box == *b"hev1"
                                || codec_box == *b"dvh1"
                                || codec_box == *b"dvhe"
                            {
                                v.format = VideoCodec::HEVC;
                                v.format_info = Some("High Efficiency Video Coding".to_string());
                                if let Some(hvcc) = Self::find_child_box(entry_data, b"hvcC") {
                                    if hvcc.len() >= 23 {
                                        let num_arrays = hvcc[22];
                                        let mut arr_off = 23;
                                        for _ in 0..num_arrays {
                                            if arr_off + 3 > hvcc.len() {
                                                break;
                                            }
                                            let nal_type = hvcc[arr_off] & 0x3F;
                                            let num_nalus = u16::from_be_bytes([
                                                hvcc[arr_off + 1],
                                                hvcc[arr_off + 2],
                                            ])
                                                as usize;
                                            arr_off += 3;
                                            for _ in 0..num_nalus {
                                                if arr_off + 2 > hvcc.len() {
                                                    break;
                                                }
                                                let nalu_len = u16::from_be_bytes([
                                                    hvcc[arr_off],
                                                    hvcc[arr_off + 1],
                                                ])
                                                    as usize;
                                                arr_off += 2;
                                                if nal_type == 33
                                                    && arr_off + nalu_len <= hvcc.len()
                                                {
                                                    let sps_bytes =
                                                        &hvcc[arr_off..arr_off + nalu_len];
                                                    if let Ok(sps) = HevcSps::parse(sps_bytes) {
                                                        v.width = sps.width;
                                                        v.height = sps.height;
                                                        v.format_profile =
                                                            Some(sps.profile_name.to_string());
                                                        v.format_level = Some(sps.level_name);
                                                        v.format_tier = Some(sps.tier.to_string());
                                                        v.bit_depth = sps.bit_depth;
                                                        v.chroma_subsampling =
                                                            Some(sps.chroma_subsampling);
                                                        v.color_range = sps.color_range;
                                                        v.color_primaries = sps.color_primaries;
                                                        v.transfer_characteristics =
                                                            sps.transfer_characteristics;
                                                        v.matrix_coefficients =
                                                            sps.matrix_coefficients;
                                                        v.hdr_format = sps.hdr_format;
                                                    }
                                                }
                                                arr_off += nalu_len;
                                            }
                                        }
                                    }
                                }

                                if let Some(dvcc) = Self::find_child_box(entry_data, b"dvcC")
                                    .or_else(|| Self::find_child_box(entry_data, b"dvvC"))
                                {
                                    if dvcc.len() >= 4 {
                                        let profile = (dvcc[2] >> 1) & 0x7F;
                                        let level =
                                            ((dvcc[2] & 0x01) << 5) | ((dvcc[3] >> 3) & 0x1F);
                                        let rpu_present = (dvcc[3] & 0x04) != 0;
                                        let el_present = (dvcc[3] & 0x02) != 0;
                                        let bl_present = (dvcc[3] & 0x01) != 0;

                                        v.dolby_vision = Some(DolbyVisionInfo {
                                            profile: DolbyVisionProfile::from_u8(profile),
                                            level,
                                            rpu_present,
                                            el_present,
                                            bl_present,
                                            bl_signal_compatibility_id: Some(profile),
                                            dm_version: Some("v2.9 / v4.0".to_string()),
                                        });
                                        v.hdr_format = Some("Dolby Vision / HDR10".to_string());
                                    }
                                }
                            } else if codec_box == *b"av01" {
                                v.format = VideoCodec::AV1;
                                v.format_info = Some("AOMedia Video 1".to_string());
                            } else if codec_box == *b"vp09" {
                                v.format = VideoCodec::VP9;
                                v.format_info = Some("Google VP9".to_string());
                            }

                            report.videos.push(v);
                        } else if handler == b"soun" {
                            let mut a = AudioTrack::default();
                            a.stream_id = track_id;
                            a.codec_id = Some(codec_str.clone());

                            if codec_box == *b"mp4a" {
                                a.format = AudioCodec::AAC;
                                a.format_info = Some("Advanced Audio Coding".to_string());
                                if let Some(esds) = Self::find_child_box(entry_data, b"esds") {
                                    if let Ok(aac) =
                                        AacInfo::parse_audio_specific_config(&esds[4..])
                                    {
                                        a.sampling_rate = aac.sampling_rate;
                                        a.channels = aac.channels;
                                        a.channel_layout = Some(aac.channel_layout);
                                        a.format_profile = Some(aac.profile.to_string());
                                    }
                                }
                            } else if codec_box == *b"ac-3" {
                                a.format = AudioCodec::AC3;
                                a.format_info = Some("Dolby Digital".to_string());
                                a.channels = 6;
                                a.channel_layout = Some(AudioChannelLayout::Surround5_1);
                                if let Some(dac3) = Self::find_child_box(entry_data, b"dac3") {
                                    if dac3.len() >= 3 {
                                        let fscod = (dac3[0] >> 6) & 0x03;
                                        let bit_rate_code =
                                            ((dac3[1] & 0x03) << 3) | ((dac3[2] >> 5) & 0x07);
                                        let bitrate_table = [
                                            32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224,
                                            256, 320, 384, 448, 512, 576, 640,
                                        ];
                                        if (bit_rate_code as usize) < bitrate_table.len() {
                                            a.bit_rate =
                                                Some(bitrate_table[bit_rate_code as usize] * 1000);
                                        }
                                        let sample_rates = [48000, 44100, 32000, 0];
                                        if (fscod as usize) < 3 {
                                            a.sampling_rate = sample_rates[fscod as usize];
                                        }
                                    }
                                }
                            } else if codec_box == *b"ec-3" {
                                a.format = AudioCodec::EAC3;
                                a.format_info = Some("Dolby Digital Plus".to_string());
                                a.channels = 6;
                                a.channel_layout = Some(AudioChannelLayout::Surround5_1);
                                if let Some(dec3) = Self::find_child_box(entry_data, b"dec3") {
                                    if dec3.len() >= 2 {
                                        let data_rate =
                                            (((dec3[0] as u64) & 0x1F) << 8) | (dec3[1] as u64);
                                        if data_rate > 0 {
                                            a.bit_rate = Some(data_rate * 1000);
                                        }
                                    }
                                }
                            }

                            report.audios.push(a);
                        } else if handler == b"subt" || handler == b"text" || handler == b"clcp" {
                            let mut s = TextTrack::default();
                            s.stream_id = track_id;
                            s.codec_id = Some(codec_str);
                            s.format = SubtitleCodec::Other("Timed Text".to_string());
                            report.texts.push(s);
                        }
                    }
                    break;
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
