use std::fs::File;
use std::io::Write;
use vuio_media_info::{MediaInfo, OutputFormat};

#[test]
fn test_cli_end_to_end_wav() {
    let temp_dir = std::env::temp_dir();
    let wav_path = temp_dir.join("test_audio.wav");

    // Write a valid WAV PCM 16-bit 44.1kHz stereo file
    let mut f = File::create(&wav_path).unwrap();
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36u32 + 88200u32).to_le_bytes()).unwrap(); // file size - 8
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap(); // Subchunk1Size
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    f.write_all(&2u16.to_le_bytes()).unwrap(); // 2 channels
    f.write_all(&44100u32.to_le_bytes()).unwrap(); // SampleRate
    f.write_all(&(44100u32 * 4).to_le_bytes()).unwrap(); // ByteRate
    f.write_all(&4u16.to_le_bytes()).unwrap(); // BlockAlign
    f.write_all(&16u16.to_le_bytes()).unwrap(); // BitsPerSample
    f.write_all(b"data").unwrap();
    f.write_all(&88200u32.to_le_bytes()).unwrap(); // 88200 bytes = 0.5s audio
    f.write_all(&vec![0u8; 88200]).unwrap();
    drop(f);

    let report = MediaInfo::open_path(&wav_path).unwrap();
    assert_eq!(report.general.format, vuio_media_info::ContainerFormat::WAV);
    assert_eq!(report.audios.len(), 1);
    assert_eq!(report.audios[0].channels, 2);
    assert_eq!(report.audios[0].sampling_rate, 44100);
    assert_eq!(report.audios[0].bit_depth, Some(16));

    // Test text format
    let txt = OutputFormat::Text.format(&report).unwrap();
    assert!(txt.contains("Wave"));
    assert!(txt.contains("PCM"));
    assert!(txt.contains("44.1 kHz"));
    assert!(txt.contains("2 channels"));

    // Test JSON format
    let json_str = OutputFormat::Json.format(&report).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["media"]["track"][0]["Format"], "Wave");

    let _ = std::fs::remove_file(wav_path);
}

/// The GUI renders `report.videos[i].format` directly, so any codec enum that serialises
/// to an object shows up as "[object Object]" instead of a codec name.
#[test]
fn test_codec_enums_serialize_as_plain_strings() {
    use vuio_media_info::{AudioCodec, SubtitleCodec, VideoCodec};

    assert_eq!(
        serde_json::to_string(&VideoCodec::ProRes).unwrap(),
        "\"ProRes\""
    );
    assert_eq!(
        serde_json::to_string(&VideoCodec::Other("apch".to_string())).unwrap(),
        "\"apch\""
    );
    assert_eq!(
        serde_json::to_string(&AudioCodec::Other("weird".to_string())).unwrap(),
        "\"weird\""
    );
    assert_eq!(
        serde_json::to_string(&SubtitleCodec::Other("Timed Text".to_string())).unwrap(),
        "\"Timed Text\""
    );

    // A serialised report must not contain an externally tagged codec object.
    let json = serde_json::to_string(&VideoCodec::Other("Unknown".to_string())).unwrap();
    assert!(
        !json.contains("Other"),
        "codec serialised as an object: {json}"
    );
}

/// ProRes profile, chroma and bit depth are fixed by the sample entry fourcc.
#[test]
fn test_prores_variants() {
    use vuio_media_info::ChromaSubsampling;
    use vuio_media_info::video::ProResVariant;

    let cases: [(&[u8; 4], &str, u8, ChromaSubsampling); 6] = [
        (b"apco", "422 Proxy", 10, ChromaSubsampling::YUV422),
        (b"apcs", "422 LT", 10, ChromaSubsampling::YUV422),
        (b"apcn", "422", 10, ChromaSubsampling::YUV422),
        (b"apch", "422 HQ", 10, ChromaSubsampling::YUV422),
        (b"ap4h", "4444", 12, ChromaSubsampling::YUV444),
        (b"ap4x", "4444 XQ", 12, ChromaSubsampling::YUV444),
    ];

    for (fourcc, profile, depth, chroma) in cases {
        let variant = ProResVariant::from_fourcc(fourcc);
        assert_eq!(variant.profile_name(), profile);
        assert_eq!(variant.bit_depth(), depth);
        assert_eq!(variant.chroma_subsampling(), chroma);
    }
}

/// EBML stores integers in the fewest bytes that fit, so a 240-pixel height arrives as a
/// single byte. Reading only two- and four-byte values silently dropped it.
#[test]
fn test_matroska_reads_narrow_ebml_integers() {
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_narrow_ebml.mkv");

    // Minimal EBML header + Segment > Tracks > TrackEntry > Video with a 1-byte height.
    let video = [
        0xB0, 0x82, 0x01, 0x40, // PixelWidth = 320 (two bytes)
        0xBA, 0x81, 0xF0, // PixelHeight = 240 (one byte)
    ];
    let mut track_entry = vec![
        0xD7, 0x81, 0x01, // TrackNumber = 1
        0x83, 0x81, 0x01, // TrackType = video
        0x86, 0x88, b'V', b'_', b'V', b'P', b'8', 0x00, 0x00, 0x00, // CodecID
    ];
    track_entry.push(0xE0);
    track_entry.push(0x80 | video.len() as u8);
    track_entry.extend_from_slice(&video);

    let mut tracks = vec![0xAE, 0x80 | track_entry.len() as u8];
    tracks.extend_from_slice(&track_entry);

    let mut segment = vec![0x16, 0x54, 0xAE, 0x6B, 0x80 | tracks.len() as u8];
    segment.extend_from_slice(&tracks);

    let mut file = File::create(&path).unwrap();
    file.write_all(&[0x1A, 0x45, 0xDF, 0xA3, 0x84, 0x42, 0x86, 0x81, 0x01])
        .unwrap();
    file.write_all(&[0x18, 0x53, 0x80, 0x67, 0x80 | segment.len() as u8])
        .unwrap();
    file.write_all(&segment).unwrap();
    drop(file);

    let report = MediaInfo::open_path(&path).unwrap();
    assert_eq!(report.videos.len(), 1);
    assert_eq!(report.videos[0].width, 320);
    assert_eq!(
        report.videos[0].height, 240,
        "one-byte PixelHeight was dropped"
    );

    let _ = std::fs::remove_file(path);
}

/// Builds a synthetic ProRes frame header with the RDD 36 field layout.
fn prores_frame(
    chroma: u8,
    interlace: u8,
    aspect: u8,
    frame_rate_code: u8,
    alpha: u8,
    version: u8,
) -> Vec<u8> {
    let mut frame = vec![0u8; 40];
    frame[0..4].copy_from_slice(&40u32.to_be_bytes());
    frame[4..8].copy_from_slice(b"icpf");
    frame[8..10].copy_from_slice(&28u16.to_be_bytes()); // frame_header_size
    frame[11] = version;
    frame[12..16].copy_from_slice(b"apl0");
    frame[16..18].copy_from_slice(&1920u16.to_be_bytes());
    frame[18..20].copy_from_slice(&1080u16.to_be_bytes());
    frame[20] = (chroma << 6) | (interlace << 2);
    frame[21] = (aspect << 4) | frame_rate_code;
    frame[22] = 1; // colour primaries BT.709
    frame[23] = 1;
    frame[24] = 1;
    frame[25] = alpha;
    frame
}

/// Every field the ProRes frame header defines, across the values RDD 36 assigns.
#[test]
fn test_prores_frame_header_spec_fields() {
    use vuio_media_info::ChromaSubsampling;
    use vuio_media_info::video::ProResHeader;

    // Chroma format: 2 is 4:2:2 and 3 is 4:4:4.
    let h = ProResHeader::parse(&prores_frame(2, 0, 0, 0, 0, 0)).unwrap();
    assert_eq!(h.chroma_subsampling, ChromaSubsampling::YUV422);
    assert_eq!(h.chroma_format, 2);
    let h = ProResHeader::parse(&prores_frame(3, 0, 0, 0, 0, 1)).unwrap();
    assert_eq!(h.chroma_subsampling, ChromaSubsampling::YUV444);
    assert_eq!(h.version, 1);

    // Interlace mode: 0 progressive, 1 top field first, 2 bottom field first.
    let h = ProResHeader::parse(&prores_frame(2, 0, 0, 0, 0, 0)).unwrap();
    assert_eq!(h.scan_type(), "Progressive");
    assert_eq!(h.scan_order(), None);
    let h = ProResHeader::parse(&prores_frame(2, 1, 0, 0, 0, 0)).unwrap();
    assert_eq!(h.scan_type(), "Interlaced");
    assert_eq!(h.scan_order(), Some("TFF"));
    let h = ProResHeader::parse(&prores_frame(2, 2, 0, 0, 0, 0)).unwrap();
    assert_eq!(h.scan_order(), Some("BFF"));

    // Frame rate codes 1 through 11.
    let expected = [
        (1u8, 24000.0 / 1001.0),
        (2, 24.0),
        (3, 25.0),
        (4, 30000.0 / 1001.0),
        (5, 30.0),
        (6, 50.0),
        (7, 60000.0 / 1001.0),
        (8, 60.0),
        (9, 100.0),
        (10, 120000.0 / 1001.0),
        (11, 120.0),
    ];
    for (code, fps) in expected {
        let h = ProResHeader::parse(&prores_frame(2, 0, 0, code, 0, 0)).unwrap();
        let got = h.frame_rate.expect("frame rate code should decode");
        assert!(
            (got - fps).abs() < 1e-6,
            "frame_rate_code {code} gave {got}, expected {fps}"
        );
    }
    // Code 0 means unknown, and 12 upwards is reserved.
    assert_eq!(
        ProResHeader::parse(&prores_frame(2, 0, 0, 0, 0, 0))
            .unwrap()
            .frame_rate,
        None
    );
    assert_eq!(
        ProResHeader::parse(&prores_frame(2, 0, 0, 12, 0, 0))
            .unwrap()
            .frame_rate,
        None
    );

    // Aspect ratio information: 1 square, 2 is 4:3, 3 is 16:9.
    assert_eq!(
        ProResHeader::parse(&prores_frame(2, 0, 2, 0, 0, 0))
            .unwrap()
            .display_aspect_ratio(),
        Some(4.0 / 3.0)
    );
    assert_eq!(
        ProResHeader::parse(&prores_frame(2, 0, 3, 0, 0, 0))
            .unwrap()
            .display_aspect_ratio(),
        Some(16.0 / 9.0)
    );
    assert_eq!(
        ProResHeader::parse(&prores_frame(2, 0, 0, 0, 0, 0))
            .unwrap()
            .display_aspect_ratio(),
        None
    );

    // Alpha channel type: 0 none, 1 8-bit, 2 16-bit.
    let h = ProResHeader::parse(&prores_frame(3, 0, 0, 0, 0, 1)).unwrap();
    assert!(!h.alpha_present);
    assert_eq!(h.alpha_bit_depth(), None);
    let h = ProResHeader::parse(&prores_frame(3, 0, 0, 0, 1, 1)).unwrap();
    assert_eq!(h.alpha_bit_depth(), Some(8));
    let h = ProResHeader::parse(&prores_frame(3, 0, 0, 0, 2, 1)).unwrap();
    assert!(h.alpha_present);
    assert_eq!(h.alpha_bit_depth(), Some(16));

    // Geometry, encoder identifier and colour tags.
    let h = ProResHeader::parse(&prores_frame(2, 0, 0, 3, 0, 0)).unwrap();
    assert_eq!((h.width, h.height), (1920, 1080));
    assert_eq!(h.encoder_identifier().as_deref(), Some("apl0"));
    assert_eq!(h.header_size, 28);
}

/// A frame without the `icpf` signature, or with an impossibly small header, is rejected.
#[test]
fn test_prores_rejects_invalid_frames() {
    use vuio_media_info::video::ProResHeader;

    let mut frame = prores_frame(2, 0, 0, 3, 0, 0);
    frame[4..8].copy_from_slice(b"junk");
    assert!(ProResHeader::parse(&frame).is_err());

    let mut frame = prores_frame(2, 0, 0, 3, 0, 0);
    frame[8..10].copy_from_slice(&4u16.to_be_bytes());
    assert!(ProResHeader::parse(&frame).is_err());

    assert!(ProResHeader::parse(&[0u8; 8]).is_err());
}

/// The FFV1 configuration record is range-coded, so its fields cannot be read directly.
#[test]
fn test_ffv1_global_header_decodes() {
    use vuio_media_info::ChromaSubsampling;
    use vuio_media_info::video::Ffv1Header;

    // CodecPrivate FFmpeg wrote for an 8-bit 4:2:0 FFV1 version 3.4 stream.
    let extradata = [
        0x56u8, 0x2B, 0x84, 0xD1, 0x9C, 0x05, 0x2F, 0x41, 0x3C, 0x60, 0x26, 0xE9, 0x5C, 0x37, 0x6F,
        0x5D, 0x1B, 0x76, 0x97, 0x9D, 0x3A, 0xC9, 0xC4, 0x20, 0x43, 0x1E, 0x8B, 0x9F, 0x55, 0x20,
        0x51, 0x2F, 0x4E, 0xF8, 0xA1, 0x68, 0x3B, 0x9B, 0x17, 0x13, 0x7C, 0x03,
    ];

    let header = Ffv1Header::parse(&extradata).expect("FFV1 global header should decode");
    assert_eq!(header.version, 3);
    assert_eq!(header.micro_version, 4);
    assert_eq!(header.version_string(), "3.4");
    assert_eq!(header.bits_per_raw_sample, 8);
    assert!(header.chroma_planes);
    assert_eq!(header.chroma_h_shift, 1);
    assert_eq!(header.chroma_v_shift, 1);
    assert_eq!(header.chroma_subsampling(), ChromaSubsampling::YUV420);
    assert!(!header.transparency);
}

/// DV's video bit rate is fixed by its DIF structure, not by the essence size, which also
/// carries audio, subcode and VAUX blocks.
#[test]
fn test_dv_video_bitrate_is_structural() {
    use vuio_media_info::video::dv_video_bitrate;

    // 525-line DV25: ten DIF sequences per frame at 30000/1001 fps.
    assert_eq!(
        dv_video_bitrate(480, 30000.0 / 1001.0, Some(120_000)),
        Some(24_417_183)
    );
    // 625-line DV25: twelve DIF sequences per frame at 25 fps.
    assert_eq!(dv_video_bitrate(576, 25.0, Some(144_000)), Some(24_441_600));
    // DVCPRO50 doubles the frame payload.
    assert_eq!(dv_video_bitrate(576, 25.0, Some(288_000)), Some(48_883_200));
    // Without a frame rate there is nothing to derive.
    assert_eq!(dv_video_bitrate(480, 0.0, Some(120_000)), None);
}

/// A ProRes frame stored in Matroska has its size and `icpf` prefix stripped, so the
/// header has to be decodable on its own.
#[test]
fn test_prores_bare_frame_header() {
    use vuio_media_info::ChromaSubsampling;
    use vuio_media_info::video::ProResHeader;

    let framed = prores_frame(2, 1, 3, 3, 0, 0);
    let with_prefix = ProResHeader::parse(&framed).unwrap();
    // Drop the 4-byte size and the `icpf` signature, as the Matroska mapping does.
    let bare = ProResHeader::parse_frame_header(&framed[8..], 40).unwrap();

    assert_eq!(bare, with_prefix);
    assert_eq!(bare.chroma_subsampling, ChromaSubsampling::YUV422);
    assert_eq!(bare.scan_order(), Some("TFF"));
    assert_eq!(bare.frame_rate, Some(25.0));
    assert_eq!(bare.display_aspect_ratio(), Some(16.0 / 9.0));
}

/// Matroska's Range element uses 1 for broadcast (limited) and 2 for full; treating 1 as
/// full inverted the range on every file that carries the element.
#[test]
fn test_matroska_colour_range_mapping() {
    use std::io::Write;
    use vuio_media_info::ColorRange;

    let build = |range: u8| {
        let colour = [0x55u8, 0xB9, 0x81, range];
        let mut video = vec![
            0xB0, 0x82, 0x01, 0x40, // PixelWidth 320
            0xBA, 0x81, 0xF0, // PixelHeight 240
        ];
        video.push(0x55);
        video.push(0xB0);
        video.push(0x80 | colour.len() as u8);
        video.extend_from_slice(&colour);

        let mut track = vec![
            0xD7, 0x81, 0x01, 0x83, 0x81, 0x01, 0x86, 0x85, b'V', b'_', b'V', b'P', b'8',
        ];
        track.push(0xE0);
        track.push(0x80 | video.len() as u8);
        track.extend_from_slice(&video);

        let mut tracks = vec![0xAE, 0x80 | track.len() as u8];
        tracks.extend_from_slice(&track);
        let mut segment = vec![0x16, 0x54, 0xAE, 0x6B, 0x80 | tracks.len() as u8];
        segment.extend_from_slice(&tracks);

        let mut file = vec![0x1A, 0x45, 0xDF, 0xA3, 0x84, 0x42, 0x86, 0x81, 0x01];
        file.push(0x18);
        file.extend_from_slice(&[0x53, 0x80, 0x67]);
        file.push(0x80 | segment.len() as u8);
        file.extend_from_slice(&segment);
        file
    };

    for (range, expected) in [(1u8, ColorRange::Limited), (2, ColorRange::Full)] {
        let path = std::env::temp_dir().join(format!("test_mkv_range_{range}.mkv"));
        File::create(&path)
            .unwrap()
            .write_all(&build(range))
            .unwrap();
        let report = MediaInfo::open_path(&path).unwrap();
        assert_eq!(report.videos.len(), 1);
        assert_eq!(
            report.videos[0].color_range,
            Some(expected),
            "Matroska Range {range} should map to {expected:?}"
        );
        let _ = std::fs::remove_file(path);
    }
}
