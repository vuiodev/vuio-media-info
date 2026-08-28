use crate::core::error::Result;
use crate::core::models::{BitstreamNode, MediaReport};
use std::io::{Read, Seek};

/// Trait for inspecting media formats and producing a unified `MediaReport`.
pub trait MediaParser: Send + Sync {
    /// Probe if this parser supports the given data prefix.
    fn probe(&self, header: &[u8]) -> bool;

    /// Parse a seekable reader into a complete MediaReport.
    fn parse_stream(&self, reader: &mut dyn ReadSeek) -> Result<MediaReport>;

    /// Parse an in-memory byte buffer.
    fn parse_buffer(&self, buffer: &[u8]) -> Result<MediaReport>;
}

/// Helper trait combining Read + Seek for media stream handling.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Trait for container demuxers that produce bitstream tree hierarchies.
pub trait BitstreamInspector {
    fn inspect_bitstream(&self, reader: &mut dyn ReadSeek) -> Result<BitstreamNode>;
}

/// Trait for formatting `MediaReport` into text, JSON, XML, etc.
pub trait Inform {
    fn inform(&self, report: &MediaReport) -> Result<String>;
}
