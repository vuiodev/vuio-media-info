use serde::{Deserialize, Serialize};

/// Hierarchical atom / box / EBML tree node for bitstream inspection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitstreamNode {
    pub name: String,
    pub description: Option<String>,
    pub offset: u64,
    pub size: u64,
    pub payload_size: Option<u64>,
    pub summary: Option<String>,
    pub children: Vec<BitstreamNode>,
}

impl BitstreamNode {
    pub fn new(name: impl Into<String>, offset: u64, size: u64) -> Self {
        Self {
            name: name.into(),
            description: None,
            offset,
            size,
            payload_size: None,
            summary: None,
            children: Vec::new(),
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn add_child(&mut self, child: BitstreamNode) {
        self.children.push(child);
    }
}
