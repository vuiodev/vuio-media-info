use serde::{Deserialize, Serialize};

/// Audio Channel Layout definitions (e.g. Stereo, 5.1, 7.1, 7.1.4 Atmos)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioChannelLayout {
    Mono,
    Stereo,
    Surround2_1,
    Surround3_0,
    Surround4_0,
    Surround5_1,
    Surround7_1,
    Spatial5_1_2,
    Spatial7_1_4,
    ObjectBased,
    Custom(String),
}

impl AudioChannelLayout {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Mono => "Mono (1.0)",
            Self::Stereo => "Stereo (2.0)",
            Self::Surround2_1 => "2.1",
            Self::Surround3_0 => "3.0",
            Self::Surround4_0 => "4.0",
            Self::Surround5_1 => "5.1 Surround (L R C LFE Ls Rs)",
            Self::Surround7_1 => "7.1 Surround (L R C LFE Ls Rs Lb Rb)",
            Self::Spatial5_1_2 => "5.1.2 Spatial / Atmos",
            Self::Spatial7_1_4 => "7.1.4 Spatial / Atmos",
            Self::ObjectBased => "Object Based",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Best-effort layout for a bare channel count, used when a container reports the
    /// channel count but no explicit speaker mask.
    pub fn from_channel_count(channels: u32) -> Option<Self> {
        Some(match channels {
            0 => return None,
            1 => Self::Mono,
            2 => Self::Stereo,
            3 => Self::Surround2_1,
            4 => Self::Surround4_0,
            6 => Self::Surround5_1,
            8 => Self::Surround7_1,
            n => Self::Custom(format!("{n} channels")),
        })
    }
}

// Serialized as a flat display string so `Custom(..)` does not become a JSON object.
impl Serialize for AudioChannelLayout {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.display_name())
    }
}

impl<'de> Deserialize<'de> for AudioChannelLayout {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Ok(match raw.as_str() {
            "Mono (1.0)" => Self::Mono,
            "Stereo (2.0)" => Self::Stereo,
            "2.1" => Self::Surround2_1,
            "3.0" => Self::Surround3_0,
            "4.0" => Self::Surround4_0,
            "5.1 Surround (L R C LFE Ls Rs)" => Self::Surround5_1,
            "7.1 Surround (L R C LFE Ls Rs Lb Rb)" => Self::Surround7_1,
            "5.1.2 Spatial / Atmos" => Self::Spatial5_1_2,
            "7.1.4 Spatial / Atmos" => Self::Spatial7_1_4,
            "Object Based" => Self::ObjectBased,
            _ => Self::Custom(raw),
        })
    }
}
