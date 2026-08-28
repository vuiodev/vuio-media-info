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
    CAF,
    DSF,
    DSDIFF,
    APE,
    WavPack,
    AIFF,
    TrueAudio,
    IVF,
    Y4M,
    AMR,
    SRT,
    ASS,
    WebVTT,
    SUP,
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
            Self::CAF => "CoreAudio Format",
            Self::DSF => "Direct Stream Digital",
            Self::DSDIFF => "DSDIFF",
            Self::APE => "Monkey's Audio",
            Self::WavPack => "WavPack",
            Self::AIFF => "AIFF",
            Self::TrueAudio => "TrueAudio",
            Self::IVF => "IVF",
            Self::Y4M => "YUV4MPEG2",
            Self::AMR => "AMR",
            Self::SRT => "SubRip Subtitle",
            Self::ASS => "SubStation Alpha Subtitle",
            Self::WebVTT => "WebVTT Subtitle",
            Self::SUP => "Blu-ray PGS Subtitle",
            Self::Unknown => "Unknown",
        }
    }

    /// Primary and common file extensions associated with this container format.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::MPEG4 => &["mp4", "m4v", "m4a", "m4b", "mov", "qt"],
            Self::Matroska => &["mkv", "mka", "mks", "mk3d"],
            Self::WebM => &["webm"],
            Self::QuickTime => &["mov", "qt"],
            Self::AVI => &["avi", "divx"],
            Self::WAV => &["wav", "wave", "bwf", "rf64"],
            Self::MPEGTS => &["ts", "m2ts", "mts", "m2t"],
            Self::MPEGPS => &["mpg", "mpeg", "vob", "evob"],
            Self::FLV => &["flv", "f4v"],
            Self::Ogg => &["ogg", "ogv", "oga", "ogx", "opus", "spx"],
            Self::MXF => &["mxf"],
            Self::ASF => &["asf", "wmv", "wma"],
            Self::FLAC => &["flac", "fla"],
            Self::MP3 => &["mp3", "mp2", "mp1", "mpa"],
            Self::AAC => &["aac", "adts"],
            Self::AC3 => &["ac3", "eac3", "ec3"],
            Self::DTS => &["dts", "dtshd", "dtsx"],
            Self::MPC => &["mpc", "mp+"],
            Self::CAF => &["caf"],
            Self::DSF => &["dsf"],
            Self::DSDIFF => &["dff"],
            Self::APE => &["ape"],
            Self::WavPack => &["wv", "wvc"],
            Self::AIFF => &["aif", "aiff", "aifc"],
            Self::TrueAudio => &["tta"],
            Self::IVF => &["ivf"],
            Self::Y4M => &["y4m"],
            Self::AMR => &["amr", "awb"],
            Self::SRT => &["srt"],
            Self::ASS => &["ass", "ssa"],
            Self::WebVTT => &["vtt"],
            Self::SUP => &["sup"],
            Self::Unknown => &[],
        }
    }

    /// Single canonical list of all media extensions recognized by the engine.
    pub fn all_supported_extensions() -> &'static [&'static str] {
        &[
            "mp4", "m4v", "m4a", "m4b", "mov", "qt", "mkv", "mka", "mks", "mk3d", "webm", "avi",
            "divx", "wav", "wave", "bwf", "rf64", "ts", "m2ts", "mts", "m2t", "mpg", "mpeg", "vob",
            "evob", "flv", "f4v", "ogg", "ogv", "oga", "ogx", "opus", "spx", "mxf", "asf", "wmv",
            "wma", "flac", "fla", "mp3", "mp2", "mp1", "mpa", "aac", "adts", "ac3", "eac3", "ec3",
            "dts", "dtshd", "dtsx", "mpc", "mp+", "caf", "dsf", "dff", "ape", "wv", "wvc", "aif",
            "aiff", "aifc", "tta", "ivf", "y4m", "amr", "awb", "srt", "ass", "ssa", "vtt", "sup",
        ]
    }

    /// Checks if a given extension (case-insensitive) is supported.
    pub fn is_supported_extension(ext: &str) -> bool {
        let clean = ext.trim_start_matches('.');
        Self::all_supported_extensions()
            .iter()
            .any(|&e| e.eq_ignore_ascii_case(clean))
    }
}

/// Known Video elementary stream codecs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VideoCodec {
    AVC,         // H.264
    HEVC,        // H.265
    AV1,         // AOMedia Video 1
    VP9,         // Google VP9
    VP8,         // Google VP8
    ProRes,      // Apple ProRes
    MPEG1Video,  // MPEG-1 Video
    MPEG2Video,  // MPEG-2 Video
    MPEG4Visual, // MPEG-4 Part 2
    VC1,         // SMPTE 421M
    VVC,         // H.266
    CineForm,    // GoPro CineForm (SMPTE ST 2073)
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
            Self::MPEG1Video => "MPEG-1 Video",
            Self::MPEG2Video => "MPEG Video",
            Self::MPEG4Visual => "MPEG-4 Visual",
            Self::VC1 => "VC-1",
            Self::VVC => "VVC",
            Self::CineForm => "CineForm",
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
            Self::MPEG1Video => "MPEG-1 Video",
            Self::MPEG2Video => "MPEG-2 Video",
            Self::MPEG4Visual => "MPEG-4 Part 2 Visual",
            Self::VC1 => "SMPTE 421M (VC-1)",
            Self::VVC => "Versatile Video Coding (H.266)",
            Self::CineForm => "GoPro CineForm HD (SMPTE ST 2073)",
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
#[allow(non_camel_case_types)]
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
    DSD,
    AC4,
    MPEGH,
    AMR_NB,
    AMR_WB,
    TTA,
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
            Self::DSD => "DSD",
            Self::AC4 => "AC-4",
            Self::MPEGH => "MPEG-H 3D Audio",
            Self::AMR_NB => "AMR-NB",
            Self::AMR_WB => "AMR-WB",
            Self::TTA => "TrueAudio",
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
            Self::AC4 => Some("Dolby AC-4"),
            _ => None,
        }
    }
}

/// Known Subtitle/Text codecs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubtitleCodec {
    SubRip,      // SRT
    ASS,         // Advanced SubStation Alpha
    SSA,         // SubStation Alpha
    PGS,         // Presentation Graphic Stream (SUP/Bluray)
    VobSub,      // DVD Subtitles (IDX/SUB)
    WebVTT,      // VTT
    TTML,        // Timed Text Markup Language
    EIA608,      // Closed Captions CEA-608
    EIA708,      // Closed Captions CEA-708
    DVBSubtitle, // DVB Subtitles
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

impl std::fmt::Display for ContainerFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::fmt::Display for SubtitleCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
