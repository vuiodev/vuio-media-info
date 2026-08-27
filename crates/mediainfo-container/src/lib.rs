#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_div_ceil,
    clippy::manual_range_contains,
    clippy::collapsible_if,
    clippy::op_ref,
    clippy::manual_checked_ops,
    clippy::manual_strip,
    clippy::trim_split_whitespace,
    clippy::manual_pattern_char_comparison,
    clippy::byte_char_slices,
    clippy::needless_range_loop,
    clippy::single_match
)]

pub mod aiff;
pub mod ape_container;
pub mod asf;
pub mod caf;
pub mod detector;
pub mod dsd;
pub mod flv;
pub mod isobmff;
pub mod ivf;
pub mod matroska;
pub mod mpeg_ts;
pub mod mxf;
pub mod ogg;
pub mod riff;
pub mod subtitles;
pub mod tta;
pub mod wavpack;
pub mod y4m;

pub use aiff::AiffDemuxer;
pub use ape_container::ApeContainerDemuxer;
pub use asf::AsfDemuxer;
pub use caf::CafDemuxer;
pub use detector::FormatDetector;
pub use dsd::DsdDemuxer;
pub use flv::FlvDemuxer;
pub use isobmff::IsobmffDemuxer;
pub use ivf::IvfDemuxer;
pub use matroska::MatroskaDemuxer;
pub use mpeg_ts::MpegTsDemuxer;
pub use mxf::MxfDemuxer;
pub use ogg::OggDemuxer;
pub use riff::RiffDemuxer;
pub use subtitles::SubtitleDemuxer;
pub use tta::TtaDemuxer;
pub use wavpack::WavpackDemuxer;
pub use y4m::Y4mDemuxer;

use mediainfo_audio::{AacInfo, Ac3Header, AmrInfo, DtsHeader, FlacStreamInfo, MpegaHeader};
use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::{ApeTag, Id3v1Tag, Id3v2Tag};

/// Top-level Container Demuxer that identifies format and extracts all tracks and metadata.
pub struct ContainerParser;

impl ContainerParser {
    pub fn parse(buffer: &[u8]) -> Result<MediaReport> {
        let format = FormatDetector::detect(buffer);

        match format {
            ContainerFormat::MPEG4 | ContainerFormat::QuickTime => {
                IsobmffDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::Matroska | ContainerFormat::WebM => {
                MatroskaDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::AVI | ContainerFormat::WAV => RiffDemuxer::parse_buffer(buffer),
            ContainerFormat::MPEGTS => MpegTsDemuxer::parse_buffer(buffer),
            ContainerFormat::Ogg => OggDemuxer::parse_buffer(buffer),
            ContainerFormat::FLV => FlvDemuxer::parse_buffer(buffer),
            ContainerFormat::ASF => AsfDemuxer::parse_buffer(buffer),
            ContainerFormat::MXF => MxfDemuxer::parse_buffer(buffer),
            ContainerFormat::CAF => CafDemuxer::parse_buffer(buffer),
            ContainerFormat::DSF | ContainerFormat::DSDIFF => DsdDemuxer::parse_buffer(buffer),
            ContainerFormat::APE => ApeContainerDemuxer::parse_buffer(buffer),
            ContainerFormat::WavPack => WavpackDemuxer::parse_buffer(buffer),
            ContainerFormat::AIFF => AiffDemuxer::parse_buffer(buffer),
            ContainerFormat::TrueAudio => TtaDemuxer::parse_buffer(buffer),
            ContainerFormat::IVF => IvfDemuxer::parse_buffer(buffer),
            ContainerFormat::Y4M => Y4mDemuxer::parse_buffer(buffer),
            ContainerFormat::AMR => Self::parse_amr_stream(buffer),
            ContainerFormat::SRT
            | ContainerFormat::ASS
            | ContainerFormat::WebVTT
            | ContainerFormat::SUP => SubtitleDemuxer::parse_buffer(buffer, format),
            ContainerFormat::FLAC => Self::parse_flac_stream(buffer),
            ContainerFormat::MP3 => Self::parse_mp3_stream(buffer),
            ContainerFormat::AAC => Self::parse_aac_stream(buffer),
            ContainerFormat::AC3 => Self::parse_ac3_stream(buffer),
            ContainerFormat::DTS => Self::parse_dts_stream(buffer),
            ContainerFormat::MPC => Self::parse_mpc_stream(buffer),
            ContainerFormat::Unknown => {
                // Try MP3 or ID3 tags fallback
                if let Ok(Some(id3)) = Id3v2Tag::parse(buffer) {
                    let mut report = MediaReport::new();
                    report.general.format = ContainerFormat::MP3;
                    report.general.file_size = buffer.len() as u64;
                    report.general.title = id3.title;
                    report.general.artist = id3.artist;
                    report.general.album = id3.album;
                    report.general.recorded_date = id3.date;
                    report.general.genre = id3.genre;
                    report.general.encoded_application = id3.encoder;
                    if id3.cover_data.is_some() {
                        report.general.cover_art_present = true;
                        report.general.cover_mime = id3.cover_mime;
                    }

                    // Try to find first MPEG audio frame after ID3
                    if let Ok(mp3) = MpegaHeader::parse(&buffer[10..]) {
                        let mut a = AudioTrack::default();
                        a.format = AudioCodec::MPEGAudioLayer3;
                        a.format_profile = Some(mp3.layer.to_string());
                        a.sampling_rate = mp3.sample_rate;
                        a.bit_rate = Some(mp3.bit_rate);
                        a.channels = mp3.channels;
                        a.channel_layout = Some(mp3.channel_layout);
                        report.audios.push(a);
                    }

                    return Ok(report);
                }

                Err(MediaInfoError::UnsupportedFormat(
                    "Unrecognized media container or bitstream format".to_string(),
                ))
            }
            _ => Err(MediaInfoError::UnsupportedFormat(format!(
                "Parser for {:?} not implemented yet",
                format
            ))),
        }
    }

    fn parse_flac_stream(data: &[u8]) -> Result<MediaReport> {
        let streaminfo = FlacStreamInfo::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::FLAC;
        report.general.file_size = data.len() as u64;
        report.general.duration_ms = Some(streaminfo.duration_ms);

        let mut a = AudioTrack::default();
        a.format = AudioCodec::FLAC;
        a.format_info = Some("Free Lossless Audio Codec".to_string());
        a.sampling_rate = streaminfo.sample_rate;
        a.channels = streaminfo.channels;
        a.channel_layout = Some(streaminfo.channel_layout);
        a.bit_depth = Some(streaminfo.bit_depth);
        a.duration_ms = Some(streaminfo.duration_ms);
        a.compression_mode = Some("Lossless".to_string());

        if streaminfo.duration_ms > 0.0 {
            let br = ((data.len() as u64 * 8) as f64 / (streaminfo.duration_ms / 1000.0)) as u64;
            a.bit_rate = Some(br);
            report.general.overall_bitrate = Some(br);
        }

        report.audios.push(a);
        Ok(report)
    }

    fn parse_mp3_stream(data: &[u8]) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MP3;
        report.general.file_size = data.len() as u64;

        let mut stream_start = 0;
        if let Ok(Some(id3)) = Id3v2Tag::parse(data) {
            report.general.title = id3.title;
            report.general.artist = id3.artist;
            report.general.album = id3.album;
            report.general.recorded_date = id3.date;
            report.general.genre = id3.genre;
            report.general.encoded_application = id3.encoder;
            if id3.cover_data.is_some() {
                report.general.cover_art_present = true;
                report.general.cover_mime = id3.cover_mime;
            }
            stream_start = id3.total_tag_size.min(data.len().saturating_sub(4));
        }

        if let Some(id3v1) = Id3v1Tag::parse(data) {
            if report.general.title.is_none() {
                report.general.title = id3v1.title;
            }
            if report.general.artist.is_none() {
                report.general.artist = id3v1.artist;
            }
            if report.general.album.is_none() {
                report.general.album = id3v1.album;
            }
        }

        // Find MP3 syncword (scanning max 64KB from stream_start)
        let mut offset = stream_start;
        let scan_limit = (stream_start + 65536).min(data.len());
        while offset + 4 <= scan_limit {
            if data[offset] == 0xFF && (data[offset + 1] & 0xE0) == 0xE0 {
                if let Ok(mp3) = MpegaHeader::parse(&data[offset..]) {
                    let mut a = AudioTrack::default();
                    a.format = AudioCodec::MPEGAudioLayer3;
                    a.format_profile = Some(mp3.layer.to_string());
                    a.sampling_rate = mp3.sample_rate;
                    a.channels = mp3.channels;
                    a.channel_layout = Some(mp3.channel_layout);

                    let audio_data_size = (data.len().saturating_sub(stream_start)) as u64;

                    let (dur_ms, calculated_bitrate) = if let (Some(frames), true) =
                        (mp3.xing_frames, mp3.sample_rate > 0)
                    {
                        let samples_per_frame = 1152.0;
                        let dur =
                            (frames as f64 * samples_per_frame / mp3.sample_rate as f64) * 1000.0;
                        let br = if dur > 0.0 {
                            let bytes = mp3.xing_bytes.map(|b| b as u64).unwrap_or(audio_data_size);
                            ((bytes * 8) as f64 / (dur / 1000.0)) as u64
                        } else {
                            mp3.bit_rate
                        };
                        (dur, br)
                    } else if mp3.bit_rate > 0 {
                        let dur = ((audio_data_size * 8) as f64 / mp3.bit_rate as f64) * 1000.0;
                        (dur, mp3.bit_rate)
                    } else {
                        (0.0, mp3.bit_rate)
                    };

                    if dur_ms > 0.0 {
                        a.duration_ms = Some(dur_ms);
                        report.general.duration_ms = Some(dur_ms);
                    }
                    a.bit_rate = Some(calculated_bitrate);
                    a.bit_rate_mode = Some(if mp3.is_vbr {
                        BitrateMode::Variable
                    } else {
                        BitrateMode::Constant
                    });
                    report.general.overall_bitrate = Some(calculated_bitrate);

                    report.audios.push(a);
                    break;
                }
            }
            offset += 1;
        }

        Ok(report)
    }

    fn parse_aac_stream(data: &[u8]) -> Result<MediaReport> {
        let aac = AacInfo::parse_adts(data)?;
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::AAC;
        report.general.file_size = data.len() as u64;

        let frame_len = if data.len() >= 6 {
            (((data[3] & 0x03) as usize) << 11)
                | ((data[4] as usize) << 3)
                | ((data[5] >> 5) as usize)
        } else {
            0
        };

        let bit_rate = if frame_len > 0 && aac.sampling_rate > 0 {
            (frame_len as u64 * 8 * aac.sampling_rate as u64) / 1024
        } else {
            128_000
        };

        let dur_ms = if bit_rate > 0 {
            (data.len() as f64 * 8.0 / bit_rate as f64) * 1000.0
        } else {
            0.0
        };

        let mut a = AudioTrack::default();
        a.format = AudioCodec::AAC;
        a.format_info = Some("Advanced Audio Coding (ADTS)".to_string());
        a.format_profile = Some(aac.profile.to_string());
        a.sampling_rate = aac.sampling_rate;
        a.channels = aac.channels;
        a.channel_layout = Some(aac.channel_layout);
        a.bit_rate = Some(bit_rate);
        if dur_ms > 0.0 {
            a.duration_ms = Some(dur_ms);
            report.general.duration_ms = Some(dur_ms);
        }
        report.general.overall_bitrate = Some(bit_rate);

        report.audios.push(a);
        Ok(report)
    }

    fn parse_ac3_stream(data: &[u8]) -> Result<MediaReport> {
        let ac3 = Ac3Header::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = if ac3.is_eac3 {
            ContainerFormat::MPEG4
        } else {
            ContainerFormat::AC3
        };
        report.general.file_size = data.len() as u64;

        let dur_ms = if ac3.bit_rate > 0 {
            (data.len() as f64 * 8.0 / ac3.bit_rate as f64) * 1000.0
        } else {
            0.0
        };

        let mut a = AudioTrack::default();
        a.format = if ac3.is_eac3 {
            AudioCodec::EAC3
        } else {
            AudioCodec::AC3
        };
        a.format_info = Some(if ac3.is_eac3 {
            "Dolby Digital Plus".to_string()
        } else {
            "Dolby Digital".to_string()
        });
        a.sampling_rate = ac3.sample_rate;
        a.bit_rate = Some(ac3.bit_rate);
        a.channels = ac3.channels;
        a.channel_layout = Some(ac3.channel_layout);
        a.dialnorm_db = Some(ac3.dialnorm_db);
        a.dolby_atmos_present = ac3.dolby_atmos_present;
        if dur_ms > 0.0 {
            a.duration_ms = Some(dur_ms);
            report.general.duration_ms = Some(dur_ms);
        }
        report.general.overall_bitrate = Some(ac3.bit_rate);

        report.audios.push(a);
        Ok(report)
    }

    fn parse_dts_stream(data: &[u8]) -> Result<MediaReport> {
        let dts = DtsHeader::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::DTS;
        report.general.file_size = data.len() as u64;

        let dur_ms = if dts.bit_rate > 0 {
            (data.len() as f64 * 8.0 / dts.bit_rate as f64) * 1000.0
        } else {
            0.0
        };

        let mut a = AudioTrack::default();
        a.format = if dts.is_dtsx {
            AudioCodec::DTSX
        } else if dts.is_dtshd_ma {
            AudioCodec::DTSHD
        } else {
            AudioCodec::DTS
        };
        a.format_info = Some(dts.profile_name.to_string());
        a.sampling_rate = dts.sample_rate;
        a.bit_rate = Some(dts.bit_rate);
        a.channels = dts.channels;
        a.channel_layout = Some(dts.channel_layout);
        a.bit_depth = Some(dts.bit_depth);
        if dur_ms > 0.0 {
            a.duration_ms = Some(dur_ms);
            report.general.duration_ms = Some(dur_ms);
        }
        report.general.overall_bitrate = Some(dts.bit_rate);

        report.audios.push(a);
        Ok(report)
    }

    fn parse_mpc_stream(data: &[u8]) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MPC;
        report.general.file_size = data.len() as u64;

        let mut a = AudioTrack::default();
        a.format = AudioCodec::MPC;
        a.format_info = Some("Musepack Audio".to_string());
        a.channels = 2;
        a.channel_layout = Some(AudioChannelLayout::Stereo);
        a.sampling_rate = 44100;

        if data.starts_with(b"MP+") && data.len() >= 28 {
            let frames = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            let sample_rate_idx = (data[8] & 0x03) as usize;
            let sample_rate = [44100, 48000, 37800, 32000][sample_rate_idx.min(3)];
            a.sampling_rate = sample_rate;
            let total_samples = frames as u64 * 1152;
            let dur_ms = (total_samples as f64 / sample_rate as f64) * 1000.0;
            if dur_ms > 0.0 {
                let br = ((data.len() as u64 * 8) as f64 / (dur_ms / 1000.0)) as u64;
                a.duration_ms = Some(dur_ms);
                a.bit_rate = Some(br);
                report.general.duration_ms = Some(dur_ms);
                report.general.overall_bitrate = Some(br);
            }
        }

        // Try extracting APEv2 tags
        if let Ok(Some(ape)) = ApeTag::parse(data) {
            report.general.title = ape.title;
            report.general.artist = ape.artist;
            report.general.album = ape.album;
            report.general.recorded_date = ape.year;
            report.general.genre = ape.genre;
            if ape.cover_data.is_some() {
                report.general.cover_art_present = true;
                report.general.cover_mime = ape.cover_mime;
            }
        }

        report.audios.push(a);
        Ok(report)
    }

    fn parse_amr_stream(data: &[u8]) -> Result<MediaReport> {
        let amr = AmrInfo::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::AMR;
        report.general.file_size = data.len() as u64;
        report.general.duration_ms = amr.duration_ms;
        report.general.overall_bitrate = amr.bit_rate;

        let mut a = AudioTrack::default();
        a.format = if amr.is_wideband {
            AudioCodec::AMR_WB
        } else {
            AudioCodec::AMR_NB
        };
        a.format_info = Some(amr.format_profile);
        a.channels = amr.channels;
        a.sampling_rate = amr.sample_rate;
        a.bit_depth = Some(amr.bit_depth);
        a.duration_ms = amr.duration_ms;
        a.bit_rate = amr.bit_rate;
        a.channel_layout = Some(AudioChannelLayout::Mono);

        report.audios.push(a);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detector() {
        assert_eq!(
            FormatDetector::detect(b"\x1A\x45\xDF\xA3\x93\x42\x82\x88matroska"),
            ContainerFormat::Matroska
        );
        assert_eq!(
            FormatDetector::detect(b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00"),
            ContainerFormat::MPEG4
        );
        assert_eq!(
            FormatDetector::detect(b"RIFF\x24\x00\x00\x00WAVEfmt "),
            ContainerFormat::WAV
        );
        assert_eq!(
            FormatDetector::detect(b"OggS\x00\x02\x00\x00"),
            ContainerFormat::Ogg
        );
        assert_eq!(
            FormatDetector::detect(b"FLV\x01\x05\x00\x00\x00\x09"),
            ContainerFormat::FLV
        );
        assert_eq!(
            FormatDetector::detect(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]),
            ContainerFormat::ASF
        );
        assert_eq!(
            FormatDetector::detect(&[0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01]),
            ContainerFormat::MXF
        );
        assert_eq!(FormatDetector::detect(b"MP+"), ContainerFormat::MPC);
    }
}
