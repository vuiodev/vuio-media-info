use serde::{Deserialize, Serialize};

/// Known container and multiplexed stream formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerFormat {
    MPEG4,
    Matroska,
    WebM,
    QuickTime,
    AVI,
    WAV,
    MPEGTS,
    MPEGPS,
    FLV,
    Ogg,
    MXF,
    ASF,
    FLAC,
    MP3,
    AAC,
    AC3,
    DTS,
    MPC,
    Unknown,
}

impl ContainerFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MPEG4 => "MPEG-4",
            Self::Matroska => "Matroska",
            Self::WebM => "WebM",
            Self::QuickTime => "QuickTime",
            Self::AVI => "AVI",
            Self::WAV => "Wave",
            Self::MPEGTS => "MPEG-TS",
            Self::MPEGPS => "MPEG-PS",
            Self::FLV => "Flash Video",
            Self::Ogg => "Ogg",
            Self::MXF => "MXF",
            Self::ASF => "Advanced Systems Format",
            Self::FLAC => "FLAC",
            Self::MP3 => "MPEG Audio",
            Self::AAC => "AAC",
            Self::AC3 => "AC-3",
            Self::DTS => "DTS",
            Self::MPC => "Musepack",
            Self::Unknown => "Unknown",
        }
    }
}

/// Known Video elementary stream codecs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VideoCodec {
    AVC,       // H.264
    HEVC,      // H.265
    AV1,       // AOMedia Video 1
    VP9,       // Google VP9
    VP8,       // Google VP8
    ProRes,    // Apple ProRes
    MPEG2Video,// MPEG-2 Video
    MPEG4Visual,// MPEG-4 Part 2
    VC1,       // SMPTE 421M
    Theora,
    DV,
    DNxHD,
    FFV1,
    Raw,
    Other(String),
}

impl VideoCodec {
    pub fn display_name(&self) -> &str {
        match self {
            Self::AVC => "AVC",
            Self::HEVC => "HEVC",
            Self::AV1 => "AV1",
            Self::VP9 => "VP9",
            Self::VP8 => "VP8",
            Self::ProRes => "ProRes",
            Self::MPEG2Video => "MPEG Video",
            Self::MPEG4Visual => "MPEG-4 Visual",
            Self::VC1 => "VC-1",
            Self::Theora => "Theora",
            Self::DV => "DV",
            Self::DNxHD => "DNxHD",
            Self::FFV1 => "FFV1",
            Self::Raw => "Raw Video",
            Self::Other(name) => name.as_str(),
        }
    }

    pub fn full_name(&self) -> &str {
        match self {
            Self::AVC => "Advanced Video Coding (H.264)",
            Self::HEVC => "High Efficiency Video Coding (H.265)",
            Self::AV1 => "AOMedia Video 1",
            Self::VP9 => "VP9",
            Self::VP8 => "VP8",
            Self::ProRes => "Apple ProRes",
            Self::MPEG2Video => "MPEG-2 Video",
            Self::MPEG4Visual => "MPEG-4 Part 2 Visual",
            Self::VC1 => "SMPTE 421M (VC-1)",
            Self::Theora => "Theora",
            Self::DV => "Digital Video",
            Self::DNxHD => "Avid DNxHD",
            Self::FFV1 => "FFmpeg Video 1",
            Self::Raw => "Raw Uncompressed Video",
            Self::Other(name) => name.as_str(),
        }
    }
}

/// Known Audio elementary stream codecs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioCodec {
    AAC,
    AC3,
    EAC3,
    TrueHD,
    DTS,
    DTSHD,
    DTSX,
    FLAC,
    MPEGAudioLayer3, // MP3
    MPEGAudioLayer2, // MP2
    MPEGAudioLayer1, // MP1
    Opus,
    Vorbis,
    PCM,
    ALAC,
    WMA,
    MonkeyAudio, // APE
    WavPack,
    MPC,
    Other(String),
}

impl AudioCodec {
    pub fn display_name(&self) -> &str {
        match self {
            Self::AAC => "AAC",
            Self::AC3 => "AC-3",
            Self::EAC3 => "E-AC-3",
            Self::TrueHD => "TrueHD",
            Self::DTS => "DTS",
            Self::DTSHD => "DTS-HD",
            Self::DTSX => "DTS:X",
            Self::FLAC => "FLAC",
            Self::MPEGAudioLayer3 => "MPEG Audio",
            Self::MPEGAudioLayer2 => "MPEG Audio",
            Self::MPEGAudioLayer1 => "MPEG Audio",
            Self::Opus => "Opus",
            Self::Vorbis => "Vorbis",
            Self::PCM => "PCM",
            Self::ALAC => "ALAC",
            Self::WMA => "WMA",
            Self::MonkeyAudio => "Monkey's Audio",
            Self::WavPack => "WavPack",
            Self::MPC => "Musepack",
            Self::Other(name) => name.as_str(),
        }
    }

    pub fn format_profile(&self) -> Option<&'static str> {
        match self {
            Self::EAC3 => Some("Dolby Digital Plus"),
            Self::MPEGAudioLayer3 => Some("Layer 3"),
            Self::MPEGAudioLayer2 => Some("Layer 2"),
            Self::MPEGAudioLayer1 => Some("Layer 1"),
            Self::TrueHD => Some("Dolby TrueHD"),
            Self::DTSHD => Some("Master Audio / High Resolution"),
            _ => None,
        }
    }
}

/// Known Subtitle/Text codecs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubtitleCodec {
    SubRip,       // SRT
    ASS,          // Advanced SubStation Alpha
    SSA,          // SubStation Alpha
    PGS,          // Presentation Graphic Stream (SUP/Bluray)
    VobSub,       // DVD Subtitles (IDX/SUB)
    WebVTT,       // VTT
    TTML,         // Timed Text Markup Language
    EIA608,       // Closed Captions CEA-608
    EIA708,       // Closed Captions CEA-708
    DVBSubtitle,  // DVB Subtitles
    Teletext,
    Other(String),
}

impl SubtitleCodec {
    pub fn display_name(&self) -> &str {
        match self {
            Self::SubRip => "SubRip",
            Self::ASS => "ASS",
            Self::SSA => "SSA",
            Self::PGS => "PGS",
            Self::VobSub => "VobSub",
            Self::WebVTT => "WebVTT",
            Self::TTML => "TTML",
            Self::EIA608 => "EIA-608",
            Self::EIA708 => "EIA-708",
            Self::DVBSubtitle => "DVB Subtitle",
            Self::Teletext => "Teletext",
            Self::Other(name) => name.as_str(),
        }
    }
}
