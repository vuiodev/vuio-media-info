use crate::core::{error::Result, models::MediaReport};

/// Formats `MediaReport` into CSV row.
pub struct CsvFormatter;

impl CsvFormatter {
    pub fn format(report: &MediaReport) -> Result<String> {
        let mut out = String::new();
        out.push_str("File,Format,FileSize,Duration_ms,Video_Codec,Resolution,FrameRate,Audio_Codec,Audio_Channels,Audio_SamplingRate\n");

        let file = report.general.file_name.clone().unwrap_or_default();
        let format = report.general.format.display_name();
        let size = report.general.file_size;
        let dur = report.general.duration_ms.unwrap_or(0.0);

        let video = report.videos.first();
        let v_codec = video.map(|v| v.format.display_name()).unwrap_or("");
        let resolution = video
            .map(|v| format!("{}x{}", v.width, v.height))
            .unwrap_or_default();
        let fps = video
            .and_then(|v| v.frame_rate)
            .map(|f| format!("{:.3}", f))
            .unwrap_or_default();

        let audio = report.audios.first();
        let a_codec = audio.map(|a| a.format.display_name()).unwrap_or("");
        let a_chans = audio.map(|a| a.channels.to_string()).unwrap_or_default();
        let a_rate = audio
            .map(|a| a.sampling_rate.to_string())
            .unwrap_or_default();

        out.push_str(&format!(
            "\"{}\",\"{}\",{},{:.0},\"{}\",\"{}\",\"{}\",\"{}\",{},{}\n",
            file, format, size, dur, v_codec, resolution, fps, a_codec, a_chans, a_rate
        ));

        Ok(out)
    }
}
