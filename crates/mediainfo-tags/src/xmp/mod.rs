use mediainfo_core::error::Result;
use std::collections::HashMap;

/// Parsed XMP (Extensible Metadata Platform) packet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XmpTag {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub create_date: Option<String>,
    pub creator: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rating: Option<String>,
    pub extra: HashMap<String, String>,
}

impl XmpTag {
    pub fn parse(data: &[u8]) -> Result<Option<Self>> {
        let text = String::from_utf8_lossy(data);
        if !text.contains("<x:xmpmeta") && !text.contains("<rdf:RDF") {
            return Ok(None);
        }

        let mut xmp = XmpTag::default();

        let extract_elem = |xml: &str, tag_name: &str| -> Option<String> {
            let patterns = [
                (format!("<{}>", tag_name), format!("</{}>", tag_name)),
                (format!("<tiff:{}>", tag_name), format!("</tiff:{}>", tag_name)),
                (format!("<exif:{}>", tag_name), format!("</exif:{}>", tag_name)),
                (format!("<dc:{}>", tag_name), format!("</dc:{}>", tag_name)),
                (format!("<xmp:{}>", tag_name), format!("</xmp:{}>", tag_name)),
                (format!("<aux:{}>", tag_name), format!("</aux:{}>", tag_name)),
            ];

            for (open, close) in &patterns {
                if let (Some(start), Some(end)) = (xml.find(open.as_str()), xml.find(close.as_str())) {
                    let val = &xml[start + open.len()..end].trim();
                    if !val.is_empty() && !val.starts_with('<') {
                        return Some(val.to_string());
                    }
                }
            }

            // Also check XML attributes: tag_name="value"
            let attr_pattern = format!("{}=\"", tag_name);
            if let Some(pos) = xml.find(&attr_pattern) {
                let start = pos + attr_pattern.len();
                if let Some(end) = xml[start..].find('"') {
                    let val = &xml[start..start + end].trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }

            None
        };

        xmp.camera_make = extract_elem(&text, "Make");
        xmp.camera_model = extract_elem(&text, "Model");
        xmp.lens_model = extract_elem(&text, "LensModel").or_else(|| extract_elem(&text, "Lens"));
        xmp.create_date = extract_elem(&text, "CreateDate").or_else(|| extract_elem(&text, "DateTimeOriginal"));
        xmp.creator = extract_elem(&text, "creator");
        xmp.title = extract_elem(&text, "title");
        xmp.description = extract_elem(&text, "description");
        xmp.rating = extract_elem(&text, "Rating");

        Ok(Some(xmp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmp_parser() {
        let xml = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:tiff="http://ns.adobe.com/tiff/1.0/"
                             xmlns:aux="http://ns.adobe.com/exif/1.0/aux/">
              <tiff:Make>Sony</tiff:Make>
              <tiff:Model>ILCE-7SM3</tiff:Model>
              <aux:LensModel>FE 24-70mm F2.8 GM II</aux:LensModel>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>"#;

        let xmp = XmpTag::parse(xml).unwrap().unwrap();
        assert_eq!(xmp.camera_make, Some("Sony".to_string()));
        assert_eq!(xmp.camera_model, Some("ILCE-7SM3".to_string()));
        assert_eq!(xmp.lens_model, Some("FE 24-70mm F2.8 GM II".to_string()));
    }
}
