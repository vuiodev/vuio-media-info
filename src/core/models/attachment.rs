use serde::{Deserialize, Serialize};

/// Attached file inside container (e.g. Matroska fonts, cover art, XML schemas).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub description: Option<String>,
    pub size: usize,
    pub data_base64: Option<String>,
}
