use mediainfo_core::{error::Result, models::MediaReport};

/// Formats `MediaReport` into a standalone, styled HTML document.
pub struct HtmlFormatter;

impl HtmlFormatter {
    pub fn format(report: &MediaReport) -> Result<String> {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n");
        html.push_str("<title>MediaInfo Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 2rem; }\n");
        html.push_str(".card { background: rgba(30, 41, 59, 0.7); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 1.5rem; margin-bottom: 1.5rem; max-width: 900px; margin-left: auto; margin-right: auto; box-shadow: 0 10px 25px rgba(0,0,0,0.5); }\n");
        html.push_str("h2 { margin-top: 0; color: #38bdf8; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 0.5rem; font-size: 1.25rem; display: flex; align-items: center; justify-content: space-between; }\n");
        html.push_str(".badge { background: #0284c7; color: white; padding: 0.2rem 0.6rem; border-radius: 9999px; font-size: 0.75rem; font-weight: bold; }\n");
        html.push_str("table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }\n");
        html.push_str("td { padding: 0.4rem 0.5rem; }\n");
        html.push_str("td.label { color: #94a3b8; width: 40%; }\n");
        html.push_str("td.value { color: #f1f5f9; font-weight: 500; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        // General
        html.push_str("<div class=\"card\">\n");
        html.push_str(&format!(
            "<h2>General <span class=\"badge\">{}</span></h2>\n",
            report.general.format.display_name()
        ));
        html.push_str("<table>\n");
        if let Some(ref name) = report.general.file_name {
            html.push_str(&format!(
                "<tr><td class=\"label\">Complete name</td><td class=\"value\">{}</td></tr>\n",
                name
            ));
        }
        html.push_str(&format!(
            "<tr><td class=\"label\">File size</td><td class=\"value\">{:.2} MB</td></tr>\n",
            report.general.file_size as f64 / (1024.0 * 1024.0)
        ));
        if let Some(dur) = report.general.duration_ms {
            html.push_str(&format!(
                "<tr><td class=\"label\">Duration</td><td class=\"value\">{:.1} s</td></tr>\n",
                dur / 1000.0
            ));
        }
        if let Some(ref title) = report.general.title {
            html.push_str(&format!(
                "<tr><td class=\"label\">Title</td><td class=\"value\">{}</td></tr>\n",
                title
            ));
        }
        html.push_str("</table>\n</div>\n");

        // Video
        for (i, v) in report.videos.iter().enumerate() {
            html.push_str("<div class=\"card\">\n");
            html.push_str(&format!(
                "<h2>Video #{} <span class=\"badge\">{}</span></h2>\n",
                i + 1,
                v.format.display_name()
            ));
            html.push_str("<table>\n");
            if v.width > 0 && v.height > 0 {
                html.push_str(&format!("<tr><td class=\"label\">Resolution</td><td class=\"value\">{} &times; {}</td></tr>\n", v.width, v.height));
            }
            if let Some(fps) = v.frame_rate {
                html.push_str(&format!("<tr><td class=\"label\">Frame rate</td><td class=\"value\">{:.3} FPS</td></tr>\n", fps));
            }
            html.push_str(&format!(
                "<tr><td class=\"label\">Bit depth</td><td class=\"value\">{} bits</td></tr>\n",
                v.bit_depth
            ));
            if let Some(ref hdr) = v.hdr_format {
                html.push_str(&format!(
                    "<tr><td class=\"label\">HDR format</td><td class=\"value\">{}</td></tr>\n",
                    hdr
                ));
            }
            html.push_str("</table>\n</div>\n");
        }

        // Audio
        for (i, a) in report.audios.iter().enumerate() {
            html.push_str("<div class=\"card\">\n");
            html.push_str(&format!(
                "<h2>Audio #{} <span class=\"badge\">{}</span></h2>\n",
                i + 1,
                a.format.display_name()
            ));
            html.push_str("<table>\n");
            html.push_str(&format!(
                "<tr><td class=\"label\">Channels</td><td class=\"value\">{} channels</td></tr>\n",
                a.channels
            ));
            html.push_str(&format!("<tr><td class=\"label\">Sampling rate</td><td class=\"value\">{:.1} kHz</td></tr>\n", a.sampling_rate as f64 / 1000.0));
            if let Some(ref title) = a.title {
                html.push_str(&format!(
                    "<tr><td class=\"label\">Title</td><td class=\"value\">{}</td></tr>\n",
                    title
                ));
            }
            html.push_str("</table>\n</div>\n");
        }

        html.push_str("</body>\n</html>\n");
        Ok(html)
    }
}
