use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// IVF (VP8 / VP9 / AV1 Elementary Stream Container) Demuxer.
pub struct IvfDemuxer;

pub const IVF_MAGIC: [u8; 4] = [b'D', b'K', b'I', b'F'];

impl IvfDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 32 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 32,
                actual: data.len(),
            });
        }

        if !data.starts_with(&IVF_MAGIC) {
            return Err(MediaInfoError::InvalidData("Not a valid IVF file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::IVF;
        report.general.file_size = data.len() as u64;

        let fourcc = &data[8..12];
        let width = u16::from_le_bytes([data[12], data[13]]) as u32;
        let height = u16::from_le_bytes([data[14], data[15]]) as u32;
        let timebase_rate = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        let timebase_scale = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        let num_frames = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

        let mut video_track = VideoTrack::default();
        video_track.width = width;
        video_track.height = height;
        video_track.frame_count = Some(num_frames as u64);

        let fourcc_str = String::from_utf8_lossy(fourcc).to_string();
        video_track.codec_id = Some(fourcc_str.clone());

        video_track.format = match fourcc {
            b"VP80" => VideoCodec::VP8,
            b"VP90" => VideoCodec::VP9,
            b"AV01" => VideoCodec::AV1,
            _ => VideoCodec::Other(fourcc_str),
        };

        if timebase_scale > 0 && timebase_rate > 0 {
            let fps = timebase_rate as f64 / timebase_scale as f64;
            video_track.frame_rate = Some(fps);
            if num_frames > 0 && fps > 0.0 {
                let duration_ms = (num_frames as f64 / fps) * 1000.0;
                video_track.duration_ms = Some(duration_ms);
                report.general.duration_ms = Some(duration_ms);

                let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
                video_track.bit_rate = Some(br);
                report.general.overall_bitrate = Some(br);
            }
        }

        report.videos.push(video_track);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivf_demuxer() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&IVF_MAGIC);
        data[4..6].copy_from_slice(&0u16.to_le_bytes()); // version
        data[6..8].copy_from_slice(&32u16.to_le_bytes()); // header size
        data[8..12].copy_from_slice(b"VP90");
        data[12..14].copy_from_slice(&1920u16.to_le_bytes());
        data[14..16].copy_from_slice(&1080u16.to_le_bytes());
        data[16..20].copy_from_slice(&30u32.to_le_bytes()); // 30 fps
        data[20..24].copy_from_slice(&1u32.to_le_bytes());
        data[24..28].copy_from_slice(&300u32.to_le_bytes()); // 300 frames = 10 sec

        let report = IvfDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::IVF);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.videos.len(), 1);
        assert_eq!(report.videos[0].width, 1920);
        assert_eq!(report.videos[0].height, 1080);
        assert_eq!(report.videos[0].format, VideoCodec::VP9);
    }
}
