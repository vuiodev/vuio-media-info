pub mod detector;
pub mod flv;
pub mod isobmff;
pub mod matroska;
pub mod mpeg_ts;
pub mod ogg;
pub mod riff;

pub use detector::FormatDetector;
pub use flv::FlvDemuxer;
pub use isobmff::IsobmffDemuxer;
pub use matroska::MatroskaDemuxer;
pub use mpeg_ts::MpegTsDemuxer;
pub use ogg::OggDemuxer;
pub use riff::RiffDemuxer;

use mediainfo_audio::{AacInfo, Ac3Header, DtsHeader, FlacStreamInfo, MpegaHeader};
use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::{Id3v1Tag, Id3v2Tag};

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
            ContainerFormat::AVI | ContainerFormat::WAV => {
                RiffDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::MPEGTS => {
                MpegTsDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::Ogg => {
                OggDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::FLV => {
                FlvDemuxer::parse_buffer(buffer)
            }
            ContainerFormat::FLAC => {
                Self::parse_flac_stream(buffer)
            }
            ContainerFormat::MP3 => {
                Self::parse_mp3_stream(buffer)
            }
            ContainerFormat::AAC => {
                Self::parse_aac_stream(buffer)
            }
            ContainerFormat::AC3 => {
                Self::parse_ac3_stream(buffer)
            }
            ContainerFormat::DTS => {
                Self::parse_dts_stream(buffer)
            }
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
            if report.general.title.is_none() { report.general.title = id3v1.title; }
            if report.general.artist.is_none() { report.general.artist = id3v1.artist; }
            if report.general.album.is_none() { report.general.album = id3v1.album; }
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
                    a.bit_rate = Some(mp3.bit_rate);
                    a.channels = mp3.channels;
                    a.channel_layout = Some(mp3.channel_layout);

                    if report.general.file_size > 0 && mp3.bit_rate > 0 {
                        let dur_ms = ((report.general.file_size * 8) as f64 / mp3.bit_rate as f64) * 1000.0;
                        a.duration_ms = Some(dur_ms);
                        report.general.duration_ms = Some(dur_ms);
                        report.general.overall_bitrate = Some(mp3.bit_rate);
                    }

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

        let mut a = AudioTrack::default();
        a.format = AudioCodec::AAC;
        a.format_info = Some("Advanced Audio Coding (ADTS)".to_string());
        a.format_profile = Some(aac.profile.to_string());
        a.sampling_rate = aac.sampling_rate;
        a.channels = aac.channels;
        a.channel_layout = Some(aac.channel_layout);

        report.audios.push(a);
        Ok(report)
    }

    fn parse_ac3_stream(data: &[u8]) -> Result<MediaReport> {
        let ac3 = Ac3Header::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = if ac3.is_eac3 { ContainerFormat::MPEG4 } else { ContainerFormat::AC3 };
        report.general.file_size = data.len() as u64;

        let mut a = AudioTrack::default();
        a.format = if ac3.is_eac3 { AudioCodec::EAC3 } else { AudioCodec::AC3 };
        a.format_info = Some(if ac3.is_eac3 { "Dolby Digital Plus".to_string() } else { "Dolby Digital".to_string() });
        a.sampling_rate = ac3.sample_rate;
        a.bit_rate = Some(ac3.bit_rate);
        a.channels = ac3.channels;
        a.channel_layout = Some(ac3.channel_layout);
        a.dialnorm_db = Some(ac3.dialnorm_db);
        a.dolby_atmos_present = ac3.dolby_atmos_present;

        report.audios.push(a);
        Ok(report)
    }

    fn parse_dts_stream(data: &[u8]) -> Result<MediaReport> {
        let dts = DtsHeader::parse(data)?;
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::DTS;
        report.general.file_size = data.len() as u64;

        let mut a = AudioTrack::default();
        a.format = if dts.is_dtsx { AudioCodec::DTSX } else if dts.is_dtshd_ma { AudioCodec::DTSHD } else { AudioCodec::DTS };
        a.format_info = Some(dts.profile_name.to_string());
        a.sampling_rate = dts.sample_rate;
        a.bit_rate = Some(dts.bit_rate);
        a.channels = dts.channels;
        a.channel_layout = Some(dts.channel_layout);
        a.bit_depth = Some(dts.bit_depth);

        report.audios.push(a);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detector() {
        assert_eq!(FormatDetector::detect(b"\x1A\x45\xDF\xA3\x93\x42\x82\x88matroska"), ContainerFormat::Matroska);
        assert_eq!(FormatDetector::detect(b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00"), ContainerFormat::MPEG4);
        assert_eq!(FormatDetector::detect(b"RIFF\x24\x00\x00\x00WAVEfmt "), ContainerFormat::WAV);
        assert_eq!(FormatDetector::detect(b"OggS\x00\x02\x00\x00"), ContainerFormat::Ogg);
        assert_eq!(FormatDetector::detect(b"FLV\x01\x05\x00\x00\x00\x09"), ContainerFormat::FLV);
    }
}
