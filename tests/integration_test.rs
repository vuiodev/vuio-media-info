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
