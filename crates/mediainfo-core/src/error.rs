use thiserror::Error;

/// Core error types for media parsing and inspection
#[derive(Error, Debug)]
pub enum MediaInfoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unexpected EOF: expected at least {expected} bytes, but only {actual} available")]
    UnexpectedEof { expected: usize, actual: usize },

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Invalid syncword: expected 0x{expected:X}, found 0x{actual:X}")]
    InvalidSyncword { expected: u32, actual: u32 },

    #[error("Unsupported format or container: {0}")]
    UnsupportedFormat(String),

    #[error("Corrupt bitstream: {0}")]
    CorruptBitstream(String),

    #[error("Bitstream reader error: {0}")]
    BitReaderError(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Format export error: {0}")]
    ExportError(String),
}

pub type Result<T> = std::result::Result<T, MediaInfoError>;
