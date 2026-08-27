use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// Flash Video (FLV) container demuxer.
pub struct FlvDemuxer;

impl FlvDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 9 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 9,
                actual: data.len(),
            });
        }

        if !data.starts_with(b"FLV") {
            return Err(MediaInfoError::InvalidData("Not a valid FLV file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::FLV;
        report.general.file_size = data.len() as u64;

        let flags = data[4];
        let has_audio = (flags & 0x04) != 0;
        let has_video = (flags & 0x01) != 0;

        if has_video {
            let mut v = VideoTrack::default();
            v.stream_id = 1;
            v.format = VideoCodec::AVC;
            v.format_info = Some("Advanced Video Coding (AVC / H.264)".to_string());
            report.videos.push(v);
        }

        if has_audio {
            let mut a = AudioTrack::default();
            a.stream_id = if has_video { 2 } else { 1 };
            a.format = AudioCodec::AAC;
            a.format_info = Some("Advanced Audio Coding (AAC)".to_string());
            report.audios.push(a);
        }

        Ok(report)
    }
}
