import { MediaReport, VideoTrack, AudioTrack, TextTrack } from "../types";

// ── formatting helpers ──────────────────────────────────────────────────────
function fmtSize(bytes: number): string {
  if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(2)} GiB (${bytes.toLocaleString()} bytes)`;
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MiB`;
  return `${(bytes / 1024).toFixed(0)} KiB`;
}

function fmtDuration(ms?: number): string {
  if (!ms || ms <= 0) return "—";
  const totalMs = Math.round(ms);
  const h = Math.floor(totalMs / 3600000);
  const m = Math.floor((totalMs % 3600000) / 60000);
  const sec = Math.floor((totalMs % 60000) / 1000);
  const millis = totalMs % 1000;

  let str = "";
  if (h > 0) {
    str = `${h}h ${m}m ${sec}s ${millis}ms`;
  } else if (m > 0) {
    str = `${m}m ${sec}s ${millis}ms`;
  } else if (sec > 0) {
    str = `${sec}s ${millis}ms`;
  } else {
    str = `${millis}ms`;
  }
  return `${str} (${fmtDurationShort(ms)})`;
}

function fmtDurationShort(ms?: number): string {
  if (!ms || ms <= 0) return "—";
  const totalMs = Math.round(ms);
  const h = Math.floor(totalMs / 3600000);
  const m = Math.floor((totalMs % 3600000) / 60000);
  const sec = Math.floor((totalMs % 60000) / 1000);
  const millis = totalMs % 1000;
  const msStr = String(millis).padStart(3, "0");
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}.${msStr}`
    : `${m}:${String(sec).padStart(2, "0")}.${msStr}`;
}

function fmtBitrate(bps?: number): string {
  if (!bps || bps <= 0) return "—";
  if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(2)} Mb/s`;
  if (bps >= 1_000) return `${(bps / 1_000).toFixed(0)} kb/s`;
  return `${bps} b/s`;
}

function fmtHz(hz: number): string {
  return hz >= 1000 ? `${(hz / 1000).toFixed(1)} kHz` : `${hz} Hz`;
}

function computedBitrate(fileSizeBytes: number, durationMs?: number): number | undefined {
  if (!durationMs || durationMs <= 0) return undefined;
  return (fileSizeBytes * 8) / (durationMs / 1000);
}

// ── detail row component ───────────────────────────────────────────────────
function row(key: string, val: string | undefined | null, highlight = false): string {
  if (!val || val === "—") return "";
  return `
    <div class="item-row">
      <span class="item-k">${key}</span>
      <span class="item-v${highlight ? " item-v-hi" : ""}">${val}</span>
    </div>`;
}

// ── render Video Collapsible Item ──────────────────────────────────────────
function renderVideoItem(vt: VideoTrack, index: number, total: number): string {
  const prores = vt.format === "ProRes";
  const extra = vt.extra || {};
  const profile = vt.format_profile
    ? `${vt.format_profile}${vt.format_level ? "@L" + vt.format_level : ""}`
    : "";
  const res = vt.width && vt.height ? `${vt.width}×${vt.height}` : "";
  const fps = vt.frame_rate ? `${vt.frame_rate.toFixed(3)} fps` : "";
  const summaryBadges = [
    res,
    fps,
    vt.bit_depth ? `${vt.bit_depth}-bit` : "",
    vt.dolby_vision_version ? "DV" : (vt.hdr_format ? "HDR" : "")
  ].filter(Boolean).join(" · ");

  return `
    <details class="track-card video-card" open>
      <summary class="track-summary">
        <div class="summary-left">
          <span class="disclosure-arrow">▶</span>
          <span class="track-title">Video ${total > 1 ? `#${index + 1}` : ""}: <strong>${vt.format}</strong></span>
        </div>
        <div class="summary-right">
          <span class="badge-tag">${summaryBadges || "Stream " + vt.stream_id}</span>
        </div>
      </summary>
      <div class="track-body">
        ${row("Format", vt.format)}
        ${row("Format profile", profile)}
        ${row("Codec ID", vt.codec_id)}
        ${row("Resolution", vt.width && vt.height ? `${vt.width} × ${vt.height} (${vt.display_aspect_ratio || "16:9"})` : "", true)}
        ${row("Frame rate", vt.frame_rate ? `${vt.frame_rate.toFixed(3)} FPS${vt.frame_rate_mode ? " (" + vt.frame_rate_mode + ")" : ""}` : "", true)}
        ${row("Frame count", vt.frame_count?.toLocaleString())}
        ${row("Bit rate", fmtBitrate(vt.bit_rate))}
        ${row("Bit rate mode", vt.bit_rate_mode)}
        ${row("Bit depth", vt.bit_depth ? `${vt.bit_depth} bits` : "")}
        ${row("Color space", vt.color_space)}
        ${row("Color encoding", vt.color_encoding)}
        ${row("Chroma subsampling", vt.chroma_subsampling)}
        ${row("Color range", vt.color_range)}
        ${row("Color primaries", vt.color_primaries)}
        ${row("Transfer char.", vt.transfer_characteristics)}
        ${row("Matrix coefficients", vt.matrix_coefficients)}
        ${prores ? row("Alpha channel", extra.Alpha_Channel) : ""}
        ${prores ? row("Alpha bit depth", extra.Alpha_BitDepth ? `${extra.Alpha_BitDepth} bits` : "") : ""}
        ${prores ? row("Picture header size", extra.ProRes_PictureHeaderSize ? `${extra.ProRes_PictureHeaderSize} bytes` : "") : ""}
        ${prores ? row("Picture data size", extra.ProRes_PictureDataSize ? `${extra.ProRes_PictureDataSize} bytes` : "") : ""}
        ${prores ? row("Slice count", extra.ProRes_SliceCount) : ""}
        ${prores ? row("Declared slice count", extra.ProRes_DeclaredSliceCount) : ""}
        ${prores ? row("Custom luma quant. matrix", extra.ProRes_CustomLumaQuantMatrix) : ""}
        ${prores ? row("Custom chroma quant. matrix", extra.ProRes_CustomChromaQuantMatrix) : ""}
        ${prores ? row("Luma quant. matrix values", extra.ProRes_LumaQuantMatrix) : ""}
        ${prores ? row("Chroma quant. matrix values", extra.ProRes_ChromaQuantMatrix) : ""}
        ${row("Mastering luminance", vt.mastering_display_luminance)}
        ${row("MaxCLL / MaxFALL", vt.maximum_content_light_level ? `${vt.maximum_content_light_level} / ${vt.maximum_frame_average_light_level ?? "?"} cd/m²` : "")}
        ${row("Dolby Vision", vt.dolby_vision_version ? `Profile ${vt.dolby_vision_profile ?? "?"}, Level ${vt.dolby_vision_level ?? "?"} (${vt.dolby_vision_version})` : "")}
        ${row("Scan type", vt.scan_type)}
        ${row("Language", vt.language)}
        ${row("Default / Forced", `${vt.default_flag ? "Yes" : "No"} / ${vt.forced_flag ? "Yes" : "No"}`)}
        ${row("Title", vt.title)}
      </div>
    </details>`;
}

// ── render Audio Collapsible Item ──────────────────────────────────────────
function renderAudioItem(at: AudioTrack, index: number, openByDefault = true): string {
  const ch = at.channels ? `${at.channels} ch` : "";
  const hz = at.sampling_rate ? fmtHz(at.sampling_rate) : "";
  const br = at.bit_rate ? fmtBitrate(at.bit_rate) : "";
  const lang = at.language ? at.language.toUpperCase() : "";
  const summaryBadges = [ch, hz, br, lang].filter(Boolean).join(" · ");

  return `
    <details class="track-card audio-card" ${openByDefault ? "open" : ""}>
      <summary class="track-summary">
        <div class="summary-left">
          <span class="disclosure-arrow">▶</span>
          <span class="track-title">Audio #${index + 1}: <strong>${at.format}</strong></span>
        </div>
        <div class="summary-right">
          <span class="badge-tag">${summaryBadges}</span>
        </div>
      </summary>
      <div class="track-body">
        ${row("Format", at.format)}
        ${row("Format profile", at.format_profile)}
        ${row("Codec ID", at.codec_id)}
        ${row("Channels", at.channels ? `${at.channels} channels (${at.channel_layout || "L R"})` : "", true)}
        ${row("Sampling rate", at.sampling_rate ? fmtHz(at.sampling_rate) : "", true)}
        ${row("Bit rate", fmtBitrate(at.bit_rate))}
        ${row("Bit rate mode", at.bit_rate_mode)}
        ${row("Bit depth", at.bit_depth ? `${at.bit_depth} bits` : "")}
        ${row("Compression", at.compression_mode)}
        ${row("Delay relative to video", at.delay_relative_to_video_ms ? `${at.delay_relative_to_video_ms} ms` : "")}
        ${row("Language", at.language)}
        ${row("Title", at.title)}
        ${row("Default / Forced", `${at.default_flag ? "Yes" : "No"} / ${at.forced_flag ? "Yes" : "No"}`)}
      </div>
    </details>`;
}

// ── render Text / Subtitle Collapsible Item ────────────────────────────────
function renderTextItem(tt: TextTrack, index: number): string {
  const lang = tt.language ? tt.language.toUpperCase() : "";
  const forced = tt.forced_flag ? "Forced" : "";
  const def = tt.default_flag ? "Default" : "";
  const summaryBadges = [lang, tt.format, def, forced].filter(Boolean).join(" · ");

  return `
    <details class="track-card text-card" open>
      <summary class="track-summary">
        <div class="summary-left">
          <span class="disclosure-arrow">▶</span>
          <span class="track-title">Sub #${index + 1}: <strong>${tt.language || tt.format}</strong></span>
        </div>
        <div class="summary-right">
          <span class="badge-tag">${summaryBadges}</span>
        </div>
      </summary>
      <div class="track-body">
        ${row("Format", tt.format)}
        ${row("Codec ID", tt.codec_id)}
        ${row("Language", tt.language)}
        ${row("Elements", tt.element_count?.toLocaleString())}
        ${row("Default / Forced", `${tt.default_flag ? "Yes" : "No"} / ${tt.forced_flag ? "Yes" : "No"}`)}
        ${row("Title", tt.title)}
      </div>
    </details>`;
}

// ── render Main Summary View ───────────────────────────────────────────────
export function renderSummaryView(report: MediaReport): string {
  const gen = report.general;
  const overallBr = gen.overall_bitrate ?? computedBitrate(gen.file_size, gen.duration_ms);
  const isAudioOnly = report.videos.length === 0;

  // General collapsible card (always open by default)
  const generalCard = `
    <details class="track-card general-card" open>
      <summary class="track-summary">
        <div class="summary-left">
          <span class="disclosure-arrow">▶</span>
          <span class="track-title">General: <strong>${gen.format}</strong></span>
        </div>
        <div class="summary-right">
          <span class="badge-tag">${fmtSize(gen.file_size)} · ${fmtDurationShort(gen.duration_ms)}</span>
        </div>
      </summary>
      <div class="track-body">
        ${row("Format", gen.format)}
        ${row("Format profile", gen.format_profile)}
        ${row("Codec ID", gen.codec_id)}
        ${row("File size", fmtSize(gen.file_size), true)}
        ${row("Duration", fmtDuration(gen.duration_ms), true)}
        ${row("Overall bitrate", fmtBitrate(overallBr), true)}
        ${row("Title", gen.title, true)}
        ${row("Artist", gen.artist, true)}
        ${row("Album", gen.album)}
        ${row("Track / Position", gen.track_position)}
        ${row("Genre", gen.genre)}
        ${row("Date", gen.recorded_date)}
        ${row("Writing app", gen.encoded_application)}
        ${row("Writing library", gen.encoded_library)}
        ${row("Cover art", gen.cover_art_present ? (gen.cover_mime || "Yes") : "")}
      </div>
    </details>`;

  // Video items (open by default)
  const videoItems = report.videos.map((vt, i) => renderVideoItem(vt, i, report.videos.length)).join("");

  // Audio items (open by default)
  const audioItems = report.audios.map((at, i) => renderAudioItem(at, i, true)).join("");

  // Subtitle items (open by default)
  const textItems = report.texts.map((tt, i) => renderTextItem(tt, i)).join("");

  // Chapters (open by default)
  let chapterItem = "";
  if (report.menu && report.menu.chapters.length > 0) {
    const chRows = report.menu.chapters.map(ch => {
      const ts = new Date(ch.timestamp_ms).toISOString().slice(11, 19);
      return `<div class="ch-row"><span class="ch-ts">${ts}</span><span class="ch-txt">${ch.title || "(untitled)"}</span></div>`;
    }).join("");
    chapterItem = `
      <details class="track-card chapter-card" open>
        <summary class="track-summary">
          <div class="summary-left">
            <span class="disclosure-arrow">▶</span>
            <span class="track-title">Chapters: <strong>${report.menu.chapters.length} markers</strong></span>
          </div>
          <div class="summary-right">
            <span class="badge-tag">${report.menu.chapters.length}</span>
          </div>
        </summary>
        <div class="track-body ch-body">
          ${chRows}
        </div>
      </details>`;
  }

  const leftHeader = isAudioOnly ? "📁 General Information" : "📁 General & 🎬 Video";
  const leftBadge = isAudioOnly ? `${gen.format}` : `${report.videos.length} stream${report.videos.length > 1 ? "s" : ""}`;
  const rightHeader = isAudioOnly ? "🔊 Audio Stream" : "🔊 Audio & 💬 Subtitles";
  const rightBadge = isAudioOnly
    ? `${report.audios.length} track${report.audios.length > 1 ? "s" : ""}`
    : `${report.audios.length} A / ${report.texts.length} S`;

  return `
    <style>
      .split-layout {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 10px;
        align-items: start;
        padding: 4px;
      }
      .col-panel {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .col-section-header {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.07em;
        padding: 4px 8px;
        border-radius: 4px;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid var(--border-subtle);
        color: var(--text-muted);
      }
      .col-section-header .count-badge {
        margin-left: auto;
        font-size: 10px;
        font-family: var(--font-mono);
        padding: 1px 6px;
        border-radius: 10px;
        background: rgba(255, 255, 255, 0.08);
      }

      /* Collapsible Track Card */
      .track-card {
        border-radius: 6px;
        background: rgba(22, 26, 38, 0.65);
        border: 1px solid var(--border-subtle);
        overflow: hidden;
        transition: border-color 0.15s ease;
      }
      .track-card:hover {
        border-color: rgba(255, 255, 255, 0.18);
      }
      .track-card[open] {
        background: rgba(18, 22, 32, 0.85);
      }

      .general-card { border-left: 3px solid #64748b; }
      .video-card { border-left: 3px solid #3b82f6; }
      .audio-card { border-left: 3px solid #10b981; }
      .text-card { border-left: 3px solid #a855f7; }
      .chapter-card { border-left: 3px solid #f59e0b; }

      /* Summary / Clickable Header */
      .track-summary {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 6px 10px;
        font-size: 12px;
        cursor: pointer;
        user-select: none;
        -webkit-user-select: none;
        background: rgba(255, 255, 255, 0.02);
        list-style: none;
      }
      .track-summary::-webkit-details-marker {
        display: none;
      }
      .track-summary:hover {
        background: rgba(255, 255, 255, 0.06);
      }

      .summary-left {
        display: flex;
        align-items: center;
        gap: 6px;
      }
      .disclosure-arrow {
        font-size: 9px;
        color: var(--text-dim);
        display: inline-block;
        transition: transform 0.18s ease;
      }
      .track-card[open] > .track-summary .disclosure-arrow {
        transform: rotate(90deg);
        color: var(--accent-blue);
      }
      .track-title {
        font-size: 11.5px;
        color: #e2e8f0;
      }
      .track-title strong {
        color: #fff;
        font-weight: 600;
      }

      .summary-right {
        display: flex;
        align-items: center;
        gap: 6px;
      }
      .badge-tag {
        font-size: 10px;
        font-family: var(--font-mono);
        color: var(--text-muted);
        background: rgba(255, 255, 255, 0.05);
        padding: 2px 6px;
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        white-space: nowrap;
      }

      /* Body Details */
      .track-body {
        padding: 4px 8px 8px;
        border-top: 1px solid rgba(255, 255, 255, 0.04);
        display: flex;
        flex-direction: column;
        gap: 1px;
      }
      .item-row {
        display: flex;
        align-items: baseline;
        padding: 2px 4px;
        border-radius: 3px;
        font-size: 11px;
      }
      .item-row:hover {
        background: rgba(255, 255, 255, 0.04);
      }
      .item-k {
        min-width: 120px;
        max-width: 120px;
        color: var(--text-muted);
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex-shrink: 0;
      }
      .item-v {
        color: #cbd5e1;
        font-family: var(--font-mono);
        word-break: break-all;
      }
      .item-v-hi {
        color: #fff;
        font-weight: 600;
      }

      /* Chapters list */
      .ch-body {
        max-height: 180px;
        overflow-y: auto;
      }
      .ch-row {
        display: flex;
        gap: 8px;
        font-size: 11px;
        padding: 2px 4px;
      }
      .ch-ts {
        color: #f59e0b;
        font-family: var(--font-mono);
        min-width: 55px;
      }
      .ch-txt {
        color: var(--text-main);
      }
    </style>

    <div class="split-layout">
      <!-- Left Column: General + Video -->
      <div class="col-panel">
        <div class="col-section-header">
          <span>${leftHeader}</span>
          <span class="count-badge">${leftBadge}</span>
        </div>
        ${generalCard}
        ${videoItems}
      </div>

      <!-- Right Column: Audio + Subtitles + Chapters -->
      <div class="col-panel">
        <div class="col-section-header">
          <span>${rightHeader}</span>
          <span class="count-badge">${rightBadge}</span>
        </div>
        ${audioItems}
        ${textItems}
        ${chapterItem}
      </div>
    </div>
  `;
}
