# Vuio Media Info

A high-performance, pure Rust rewrite of [MediaInfoLib](https://github.com/MediaArea/MediaInfoLib).

Provides technical and tag information about video and audio files with memory safety, zero-copy bitstream parsing, and multithreaded batch processing.

---

## Key Features

* **Pure Rust & Memory Safe**: 100% safe, modern Rust without global state or unsafe pointer arithmetic.
* **Zero-Copy Bitstream Engine**: Sub-byte `MsbBitReader` and `LsbBitReader` with Exp-Golomb decoding and automatic NAL emulation prevention unescaping.
* **Extensive Container Support**:
  * **ISOBMFF / MP4 / QuickTime** (`.mp4`, `.mov`, `.m4v`, `.m4a`)
  * **Matroska / WebM** (`.mkv`, `.mka`, `.mks`, `.webm`)
  * **RIFF** (`.avi`, `.wav`)
  * **MPEG Transport Stream** (`.ts`, `.m2ts`)
  * **Ogg** (`.ogg`, `.opus`)
  * **Flash Video** (`.flv`)
* **Deep Video Bitstream Inspection**:
  * **H.264 / AVC**: Full SPS parsing (profiles, levels, SAR, VUI, chroma subsampling, bit depth, cropping).
  * **H.265 / HEVC**: Profile Tier Level, VUI, SMPTE ST 2086 HDR10, HLG.
  * **Dolby Vision**: Complete RPU (Reference Processing Unit) metadata decoding (Profiles 4, 5, 7, 8, 9, levels, BL+EL+RPU).
  * **AV1**: Sequence Header OBU parsing.
  * **VP9**: Frame header & color space parsing.
  * **Apple ProRes**: Frame header & colorimetry metadata.
  * **MPEG-1/2 Video**: Sequence header parsing.
* **Audio Bitstream Inspection**:
  * **AAC**: ADTS headers and `AudioSpecificConfig` (LC, HE-AAC v1/v2, SBR, PS).
  * **Dolby Digital / Plus**: AC-3 and E-AC-3 frame headers, Dialog Normalization, Joint Object Coding (Dolby Atmos).
  * **DTS / DTS-HD**: DTS Core, Master Audio, DTS:X.
  * **FLAC**: STREAMINFO metadata block.
  * **MPEG Audio**: MP1 / MP2 / MP3 frames with Xing / VBRI VBR headers.
  * **Opus**: `OpusHead` packets.
  * **PCM / LPCM**: Linear PCM formats.
* **Rich Tag & Metadata Parsing**:
  * ID3v1, ID3v2.2, ID3v2.3, ID3v2.4 (synchsafe integers & APIC cover art extraction).
  * Vorbis Comments.
  * Apple / iTunes `ilst` atoms & cover art.
  * EXIF / TIFF tags.
* **Multi-Format Reporting**:
  * Classic aligned 2-column MediaInfo text
  * Standard MediaInfo JSON schema (`{"media": {"track": [...]}}`)
  * MediaInfo 2.0 XML
  * CSV batch export
  * Standalone styled HTML reports
* **High Performance**: Memory-mapped file I/O (`memmap2`) and multicore parallel directory scanning (`rayon`).

---

## Workspace Architecture

The project is structured as a modular Cargo workspace:

```
mediainfo/
├── crates/
│   ├── mediainfo-core/       # Core traits, error types, bitstream readers, unified data models
│   ├── mediainfo-video/      # Video codec parsers (AVC, HEVC, AV1, VP9, ProRes, MPEG-2)
│   ├── mediainfo-audio/      # Audio codec parsers (AAC, AC-3/E-AC-3, DTS, FLAC, MP3, Opus)
│   ├── mediainfo-tags/       # Metadata tag parsers (ID3v1, ID3v2, Vorbis, iTunes, EXIF)
│   ├── mediainfo-container/  # Container demuxers (ISOBMFF, Matroska, RIFF, MPEG-TS, Ogg, FLV)
│   ├── mediainfo-format/     # Report formatters (Text, JSON, XML, CSV, HTML)
│   └── mediainfo-diff/       # Differential testing suite vs C++ MediaInfoLib
├── src/
│   ├── lib.rs                # Public high-level facade API
│   └── main.rs               # Command-line interface (CLI)
└── Cargo.toml                # Root workspace manifest
```

---

## CLI Usage

### Basic Inspection (Classic Text)
```bash
mediainfo movie.mkv
```

### JSON Output
```bash
mediainfo -O json movie.mp4
```

### XML Output
```bash
mediainfo -O xml sample.m2ts
```

### CSV Batch Summary
```bash
mediainfo -O csv -r /path/to/media/library/ > library_report.csv
```

### Standalone HTML Report
```bash
mediainfo -O html video.mkv > report.html
```

---

## Rust Library API

Add `mediainfo` to your `Cargo.toml`:

```toml
[dependencies]
mediainfo = { path = "path/to/mediainfo" }
```

### Example Usage:

```rust
use mediainfo::{MediaInfo, OutputFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Analyze a media file on disk (auto memory-mapped)
    let report = MediaInfo::open_path("sample.mkv")?;

    // 2. Access strongly-typed properties
    println!("Container format: {}", report.general.format.display_name());
    if let Some(dur) = report.general.duration_ms {
        println!("Duration: {:.2} seconds", dur / 1000.0);
    }

    for video in &report.videos {
        println!(
            "Video track #{}: {} {}x{} @ {:.3} fps",
            video.stream_id,
            video.format.display_name(),
            video.width,
            video.height,
            video.frame_rate.unwrap_or(0.0)
        );
        if let Some(ref hdr) = video.hdr_format {
            println!("  HDR format: {}", hdr);
        }
    }

    for audio in &report.audios {
        println!(
            "Audio track #{}: {} {} channels @ {} Hz",
            audio.stream_id,
            audio.format.display_name(),
            audio.channels,
            audio.sampling_rate
        );
    }

    // 3. Export to JSON, XML, Text, CSV, or HTML
    let json_output = OutputFormat::Json.format(&report)?;
    println!("{}", json_output);

    Ok(())
}
```

---

## License

MIT or Apache 2.0

---

## Attribution

Part of this work is based on MediaInfoLib by MediaArea.net SARL.
See NOTICE for original MediaInfoLib license.
