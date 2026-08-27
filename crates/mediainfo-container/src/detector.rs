use mediainfo_core::types::ContainerFormat;

/// Fast sniffer that inspects the initial byte slice to identify the container format.
pub struct FormatDetector;

impl FormatDetector {
    pub fn detect(header: &[u8]) -> ContainerFormat {
        if header.len() < 3 {
            return ContainerFormat::Unknown;
        }

        let prefix_len = header.len().min(4096);
        let prefix = &header[..prefix_len];

        // Matroska / WebM EBML Header: 0x1A 0x45 0xDF 0xA3
        if prefix.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
            if prefix.windows(4).any(|w| w == b"webm") {
                return ContainerFormat::WebM;
            }
            return ContainerFormat::Matroska;
        }

        // ISOBMFF / MP4 / QuickTime: [size:4] "ftyp" or "moov" or "mdat" or "wide"
        if prefix.len() >= 8 {
            let box_type = &prefix[4..8];
            if box_type == b"ftyp" {
                if prefix.len() >= 12 {
                    let brand = &prefix[8..12];
                    if brand == b"qt  " || brand == b"moov" {
                        return ContainerFormat::QuickTime;
                    }
                }
                return ContainerFormat::MPEG4;
            } else if box_type == b"moov" || box_type == b"mdat" || box_type == b"wide" || box_type == b"skip" {
                return ContainerFormat::MPEG4;
            }
        }

        // RIFF (AVI / WAV) or RF64 / BW64
        if prefix.starts_with(b"RIFF") || prefix.starts_with(b"RIFX") || prefix.starts_with(b"RF64") || prefix.starts_with(b"BW64") {
            if prefix.len() >= 12 {
                let form_type = &prefix[8..12];
                if form_type == b"AVI " || form_type == b"AVIX" {
                    return ContainerFormat::AVI;
                } else if form_type == b"WAVE" {
                    return ContainerFormat::WAV;
                }
            }
            return ContainerFormat::WAV;
        }

        // MPEG-TS: 0x47 sync byte every 188 bytes
        if prefix[0] == 0x47 {
            if prefix.len() >= 188 * 3 {
                if prefix[188] == 0x47 && prefix[376] == 0x47 {
                    return ContainerFormat::MPEGTS;
                }
            } else {
                return ContainerFormat::MPEGTS;
            }
        }

        // MPEG-PS: 0x00 0x00 0x01 0xBA (Pack Header)
        if prefix.starts_with(&[0x00, 0x00, 0x01, 0xBA]) {
            return ContainerFormat::MPEGPS;
        }

        // Ogg Container: "OggS"
        if prefix.starts_with(b"OggS") {
            return ContainerFormat::Ogg;
        }

        // FLV Container: "FLV\x01"
        if prefix.starts_with(b"FLV") {
            return ContainerFormat::FLV;
        }

        // FLAC Stream: "fLaC"
        if prefix.starts_with(b"fLaC") {
            return ContainerFormat::FLAC;
        }

        // MP3 with ID3v2: "ID3"
        if prefix.starts_with(b"ID3") {
            return ContainerFormat::MP3;
        }

        // Raw AAC ADTS: 0xFFF syncword
        if prefix.len() >= 2 && prefix[0] == 0xFF && (prefix[1] & 0xF6) == 0xF0 {
            return ContainerFormat::AAC;
        }

        // Raw AC-3: 0x0B77 syncword
        if prefix.starts_with(&[0x0B, 0x77]) || prefix.starts_with(&[0x77, 0x0B]) {
            return ContainerFormat::AC3;
        }

        // Raw DTS: 0x7FFE8001 or 0x1FFFE800 or 0xFE7F0180
        if prefix.starts_with(&[0x7F, 0xFE, 0x80, 0x01])
            || prefix.starts_with(&[0x1F, 0xFF, 0xE8, 0x00])
            || prefix.starts_with(&[0xFE, 0x7F, 0x01, 0x80])
        {
            return ContainerFormat::DTS;
        }

        // MPEG-1/2 Audio syncword (0xFFE or 0xFFF)
        if prefix.len() >= 2 && prefix[0] == 0xFF && (prefix[1] & 0xE0) == 0xE0 {
            return ContainerFormat::MP3;
        }

        // Musepack (MPC): "MP+" (SV7) or "MPCK" (SV8)
        if prefix.starts_with(b"MP+") || prefix.starts_with(b"MPCK") {
            return ContainerFormat::MPC;
        }

        // Material Exchange Format (MXF - SMPTE 377M): 06 0E 2B 34
        if prefix.starts_with(&[0x06, 0x0E, 0x2B, 0x34]) {
            return ContainerFormat::MXF;
        }

        // Advanced Systems Format (ASF / WMA / WMV): 30 26 B2 75 8E 66 CF 11
        if prefix.starts_with(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
            return ContainerFormat::ASF;
        }

        ContainerFormat::Unknown
    }
}
