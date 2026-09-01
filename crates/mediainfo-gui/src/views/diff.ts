import { MediaReport } from "../types";

export interface CompareSlot {
  path: string;
  name: string;
  report: MediaReport;
}

export type CompareFilter = "all" | "diff" | "identical";

export interface CompareFieldRow {
  category: string;
  field: string;
  values: string[];
  isDiff: boolean;
}

export const MAX_COMPARE_FILES = 6;

const SLOT_COLORS = [
  { name: "File 1", color: "#3b82f6", lightColor: "#60a5fa", bg: "rgba(59, 130, 246, 0.12)", border: "rgba(59, 130, 246, 0.4)" },
  { name: "File 2", color: "#8b5cf6", lightColor: "#c084fc", bg: "rgba(139, 92, 246, 0.12)", border: "rgba(139, 92, 246, 0.4)" },
  { name: "File 3", color: "#10b981", lightColor: "#34d399", bg: "rgba(16, 185, 129, 0.12)", border: "rgba(16, 185, 129, 0.4)" },
  { name: "File 4", color: "#f59e0b", lightColor: "#fbbf24", bg: "rgba(245, 158, 11, 0.12)", border: "rgba(245, 158, 11, 0.4)" },
  { name: "File 5", color: "#06b6d4", lightColor: "#22d3ee", bg: "rgba(6, 182, 212, 0.12)", border: "rgba(6, 182, 212, 0.4)" },
  { name: "File 6", color: "#ec4899", lightColor: "#f472b6", bg: "rgba(236, 72, 153, 0.12)", border: "rgba(236, 72, 153, 0.4)" },
];

function formatBytes(bytes?: number): string {
  if (!bytes || bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

function formatDuration(ms?: number): string {
  if (!ms || isNaN(ms)) return "—";
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const millis = Math.floor(ms % 1000);

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
  }
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
}

function formatBitrate(bps?: number): string {
  if (!bps || isNaN(bps)) return "—";
  if (bps >= 1_000_000) {
    return `${(bps / 1_000_000).toFixed(2)} Mbps`;
  }
  return `${Math.round(bps / 1000)} kbps`;
}

export function buildComparisonRows(slots: CompareSlot[]): CompareFieldRow[] {
  if (slots.length < 2) return [];

  const rows: CompareFieldRow[] = [];

  function addRow(
    category: string,
    field: string,
    extractor: (r: MediaReport) => string | number | boolean | undefined | null
  ) {
    const rawValues = slots.map((s) => {
      const v = extractor(s.report);
      if (v === undefined || v === null || v === "") return "—";
      if (typeof v === "boolean") return v ? "Yes" : "No";
      return String(v);
    });

    // Check if all values are empty placeholder
    const allEmpty = rawValues.every((v) => v === "—");
    if (allEmpty) return;

    // Check if all values are identical
    const first = rawValues[0];
    const isDiff = rawValues.some((v) => v !== first);

    rows.push({
      category,
      field,
      values: rawValues,
      isDiff,
    });
  }

  // 1. General Track
  addRow("General", "File Name", (r) => r.general?.file_name || r.general?.file_path?.split("/").pop());
  addRow("General", "Container Format", (r) => r.general?.format);
  addRow("General", "Format Profile", (r) => r.general?.format_profile);
  addRow("General", "Format Version", (r) => r.general?.format_version);
  addRow("General", "Codec ID", (r) => r.general?.codec_id);
  addRow("General", "File Size", (r) => r.general?.file_size ? `${formatBytes(r.general.file_size)} (${r.general.file_size.toLocaleString()} B)` : undefined);
  addRow("General", "Duration", (r) => r.general?.duration_ms ? formatDuration(r.general.duration_ms) : undefined);
  addRow("General", "Overall Bitrate", (r) => r.general?.overall_bitrate ? formatBitrate(r.general.overall_bitrate) : undefined);
  addRow("General", "Encoded Application", (r) => r.general?.encoded_application);
  addRow("General", "Encoded Library", (r) => r.general?.encoded_library);
  addRow("General", "Title", (r) => r.general?.title);
  addRow("General", "Artist", (r) => r.general?.artist);
  addRow("General", "Album", (r) => r.general?.album);
  addRow("General", "Genre", (r) => r.general?.genre);
  addRow("General", "Recorded Date", (r) => r.general?.recorded_date);
  addRow("General", "Streamable", (r) => r.general?.is_streamable !== undefined ? (r.general.is_streamable ? "Yes" : "No") : undefined);
  addRow("General", "Cover Art Present", (r) => r.general?.cover_art_present ? "Yes" : "No");

  // 2. Video Tracks
  const maxVideos = Math.max(...slots.map((s) => s.report.videos?.length || 0), 0);
  if (maxVideos > 0) {
    addRow("Video Tracks", "Total Video Tracks", (r) => r.videos?.length || 0);

    for (let i = 0; i < maxVideos; i++) {
      const cat = `Video Track #${i + 1}`;
      addRow(cat, "Format / Codec", (r) => r.videos?.[i]?.format);
      addRow(cat, "Format Info", (r) => r.videos?.[i]?.format_info);
      addRow(cat, "Format Profile", (r) => r.videos?.[i]?.format_profile);
      addRow(cat, "Format Level", (r) => r.videos?.[i]?.format_level);
      addRow(cat, "Codec ID", (r) => r.videos?.[i]?.codec_id);
      addRow(cat, "Resolution", (r) => r.videos?.[i] ? `${r.videos[i].width} × ${r.videos[i].height}` : undefined);
      addRow(cat, "Display Aspect Ratio", (r) => r.videos?.[i]?.display_aspect_ratio);
      addRow(cat, "Frame Rate", (r) => r.videos?.[i]?.frame_rate ? `${r.videos[i].frame_rate?.toFixed(3)} fps` : undefined);
      addRow(cat, "Frame Rate Mode", (r) => r.videos?.[i]?.frame_rate_mode);
      addRow(cat, "Frame Count", (r) => r.videos?.[i]?.frame_count?.toLocaleString());
      addRow(cat, "Bitrate", (r) => r.videos?.[i]?.bit_rate ? formatBitrate(r.videos[i].bit_rate) : undefined);
      addRow(cat, "Bitrate Mode", (r) => r.videos?.[i]?.bit_rate_mode);
      addRow(cat, "Bit Depth", (r) => r.videos?.[i]?.bit_depth ? `${r.videos[i].bit_depth} bit` : undefined);
      addRow(cat, "Color Space", (r) => r.videos?.[i]?.color_space);
      addRow(cat, "Chroma Subsampling", (r) => r.videos?.[i]?.chroma_subsampling);
      addRow(cat, "Color Primaries", (r) => r.videos?.[i]?.color_primaries);
      addRow(cat, "Transfer Characteristics", (r) => r.videos?.[i]?.transfer_characteristics);
      addRow(cat, "Matrix Coefficients", (r) => r.videos?.[i]?.matrix_coefficients);
      addRow(cat, "Color Range", (r) => r.videos?.[i]?.color_range);
      addRow(cat, "Scan Type", (r) => r.videos?.[i]?.scan_type);
      addRow(cat, "HDR Format", (r) => r.videos?.[i]?.hdr_format);
      addRow(cat, "Dolby Vision Profile", (r) => r.videos?.[i]?.dolby_vision_profile);
      addRow(cat, "Dolby Vision Level", (r) => r.videos?.[i]?.dolby_vision_level);
      addRow(cat, "Dolby Vision RPU Present", (r) => r.videos?.[i]?.dolby_vision_rpu_present !== undefined ? (r.videos[i].dolby_vision_rpu_present ? "Yes" : "No") : undefined);
      addRow(cat, "Max CLL / Max FALL", (r) => {
        const v = r.videos?.[i];
        if (v && (v.maximum_content_light_level || v.maximum_frame_average_light_level)) {
          return `${v.maximum_content_light_level || 0} / ${v.maximum_frame_average_light_level || 0} cd/m²`;
        }
        return undefined;
      });
      addRow(cat, "Track Title", (r) => r.videos?.[i]?.title);
      addRow(cat, "Language", (r) => r.videos?.[i]?.language);
      addRow(cat, "Default / Forced", (r) => {
        const v = r.videos?.[i];
        if (!v) return undefined;
        return `${v.default_flag ? "Default" : "No"} / ${v.forced_flag ? "Forced" : "No"}`;
      });

      // Codec-specific bitstream and ProRes metadata (all dynamic extra fields)
      const extraKeys = new Set<string>();
      for (const s of slots) {
        const v = s.report.videos?.[i];
        if (v?.extra) {
          for (const k of Object.keys(v.extra)) {
            extraKeys.add(k);
          }
        }
      }
      for (const key of Array.from(extraKeys).sort()) {
        const displayKey = key.replace(/_/g, " ");
        addRow(cat, displayKey, (r) => r.videos?.[i]?.extra?.[key]);
      }
    }
  }

  // 3. Audio Tracks
  const maxAudios = Math.max(...slots.map((s) => s.report.audios?.length || 0), 0);
  if (maxAudios > 0) {
    addRow("Audio Tracks", "Total Audio Tracks", (r) => r.audios?.length || 0);

    for (let i = 0; i < maxAudios; i++) {
      const cat = `Audio Track #${i + 1}`;
      addRow(cat, "Format / Codec", (r) => r.audios?.[i]?.format);
      addRow(cat, "Format Info", (r) => r.audios?.[i]?.format_info);
      addRow(cat, "Format Profile", (r) => r.audios?.[i]?.format_profile);
      addRow(cat, "Codec ID", (r) => r.audios?.[i]?.codec_id);
      addRow(cat, "Channels", (r) => {
        const a = r.audios?.[i];
        if (!a) return undefined;
        return `${a.channels} ch${a.channel_layout ? ` (${a.channel_layout})` : ""}`;
      });
      addRow(cat, "Sampling Rate", (r) => r.audios?.[i]?.sampling_rate ? `${r.audios[i].sampling_rate.toLocaleString()} Hz` : undefined);
      addRow(cat, "Bit Depth", (r) => r.audios?.[i]?.bit_depth ? `${r.audios[i].bit_depth} bit` : undefined);
      addRow(cat, "Bitrate", (r) => r.audios?.[i]?.bit_rate ? formatBitrate(r.audios[i].bit_rate) : undefined);
      addRow(cat, "Bitrate Mode", (r) => r.audios?.[i]?.bit_rate_mode);
      addRow(cat, "Compression Mode", (r) => r.audios?.[i]?.compression_mode);
      addRow(cat, "Delay relative to Video", (r) => r.audios?.[i]?.delay_relative_to_video_ms !== undefined ? `${r.audios[i].delay_relative_to_video_ms} ms` : undefined);
      addRow(cat, "Dolby Atmos Present", (r) => r.audios?.[i]?.dolby_atmos_present ? "Yes" : "No");
      addRow(cat, "DTS:X Present", (r) => r.audios?.[i]?.dts_x_present ? "Yes" : "No");
      addRow(cat, "Track Title", (r) => r.audios?.[i]?.title);
      addRow(cat, "Language", (r) => r.audios?.[i]?.language);
      addRow(cat, "Default / Forced", (r) => {
        const a = r.audios?.[i];
        if (!a) return undefined;
        return `${a.default_flag ? "Default" : "No"} / ${a.forced_flag ? "Forced" : "No"}`;
      });
    }
  }

  // 4. Subtitle Tracks
  const maxTexts = Math.max(...slots.map((s) => s.report.texts?.length || 0), 0);
  if (maxTexts > 0) {
    addRow("Subtitle Tracks", "Total Subtitle Tracks", (r) => r.texts?.length || 0);

    for (let i = 0; i < maxTexts; i++) {
      const cat = `Subtitle Track #${i + 1}`;
      addRow(cat, "Format / Codec", (r) => r.texts?.[i]?.format);
      addRow(cat, "Codec ID", (r) => r.texts?.[i]?.codec_id);
      addRow(cat, "Element Count", (r) => r.texts?.[i]?.element_count?.toLocaleString());
      addRow(cat, "Track Title", (r) => r.texts?.[i]?.title);
      addRow(cat, "Language", (r) => r.texts?.[i]?.language);
      addRow(cat, "Default / Forced", (r) => {
        const t = r.texts?.[i];
        if (!t) return undefined;
        return `${t.default_flag ? "Default" : "No"} / ${t.forced_flag ? "Forced" : "No"}`;
      });
    }
  }

  // 5. Chapters / Menu
  const hasChapters = slots.some((s) => (s.report.menu?.chapters?.length || 0) > 0);
  if (hasChapters) {
    addRow("Chapters / Menu", "Total Chapters", (r) => r.menu?.chapters?.length || 0);
    addRow("Chapters / Menu", "First Chapter", (r) => r.menu?.chapters?.[0]?.title || undefined);
    addRow("Chapters / Menu", "Last Chapter", (r) => {
      const chs = r.menu?.chapters;
      return chs && chs.length > 0 ? chs[chs.length - 1].title : undefined;
    });
  }

  // 6. Attachments
  const hasAttachments = slots.some((s) => (s.report.attachments?.length || 0) > 0);
  if (hasAttachments) {
    addRow("Attachments", "Attachment Count", (r) => r.attachments?.length || 0);
    addRow("Attachments", "Attached Files", (r) => r.attachments?.map((a) => a.file_name).join(", ") || undefined);
  }

  return rows;
}

export function renderDiffView(
  slots: CompareSlot[],
  filter: CompareFilter = "all",
  searchQuery: string = "",
  batchReports: MediaReport[] = []
): string {
  // Empty state if < 2 files selected
  if (slots.length < 2) {
    const quickAddFromBatch = batchReports.length > 0 ? `
      <div style="margin-top: 20px; width: 100%; max-width: 620px;">
        <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--text-muted); margin-bottom: 8px;">
          Or quick-add from currently open files (${batchReports.length})
        </div>
        <div style="display: flex; flex-wrap: wrap; gap: 6px; justify-content: center;">
          ${batchReports.slice(0, 12).map((r, i) => {
            const name = r.general?.file_name || r.general?.file_path?.split("/").pop() || `File #${i + 1}`;
            const fmt = r.general?.format || "";
            const isAlreadyAdded = slots.some((s) => s.path === r.general?.file_path);
            return `
              <button class="btn btn-stepper btn-quick-add-compare ${isAlreadyAdded ? "disabled" : ""}" data-batch-index="${i}" ${isAlreadyAdded ? "disabled" : ""}>
                + ${name} (${fmt})
              </button>
            `;
          }).join("")}
        </div>
      </div>
    ` : "";

    return `
      <div class="empty-state-card" id="diff-open-card">
        <div style="font-size: 36px; margin-bottom: 12px;">⚖️</div>
        <div style="font-weight: 700; font-size: 16px; color: #fff;">Multi-File Media Comparison</div>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 6px; max-width: 480px; text-align: center; line-height: 1.6;">
          Compare <strong>2 to 6 files simultaneously</strong> side-by-side. Inspect technical parameters, codecs, resolutions, bitrates, audio tracks, and colorimetry with differential filtering.
        </div>
        
        <div style="display: flex; gap: 12px; margin-top: 20px; flex-wrap: wrap; justify-content: center;">
          <button id="compare-add-slot-btn" class="btn btn-primary">📂 Select Files to Compare</button>
        </div>

        ${quickAddFromBatch}
      </div>
    `;
  }

  const allRows = buildComparisonRows(slots);
  const diffRowsCount = allRows.filter((r) => r.isDiff).length;
  const identicalRowsCount = allRows.length - diffRowsCount;

  // Filter rows based on selected filter mode and search query
  const query = searchQuery.trim().toLowerCase();
  const visibleRows = allRows.filter((r) => {
    if (filter === "diff" && !r.isDiff) return false;
    if (filter === "identical" && r.isDiff) return false;
    if (query) {
      const matchCat = r.category.toLowerCase().includes(query);
      const matchField = r.field.toLowerCase().includes(query);
      const matchValue = r.values.some((v) => v.toLowerCase().includes(query));
      if (!matchCat && !matchField && !matchValue) return false;
    }
    return true;
  });

  // Group visible rows by Category
  const categories: Record<string, CompareFieldRow[]> = {};
  for (const r of visibleRows) {
    if (!categories[r.category]) categories[r.category] = [];
    categories[r.category].push(r);
  }

  // Slot cards HTML
  const slotCardsHtml = `
    <div class="compare-slots-grid" style="grid-template-columns: repeat(${Math.min(slots.length + (slots.length < MAX_COMPARE_FILES ? 1 : 0), 6)}, minmax(0, 1fr));">
      ${slots.map((s, idx) => {
        const theme = SLOT_COLORS[idx] || SLOT_COLORS[0];
        const fname = s.name || s.path.split("/").pop() || `File #${idx + 1}`;
        const fmt = s.report.general?.format || "Unknown";
        const vTrack = s.report.videos?.[0];
        const aTrack = s.report.audios?.[0];
        const summary = [
          vTrack ? `${vTrack.format} ${vTrack.width}x${vTrack.height}` : null,
          aTrack ? `${aTrack.format} ${aTrack.channels}ch` : null,
        ].filter(Boolean).join(" • ");

        return `
          <div class="compare-slot-card" style="border-top: 3px solid ${theme.color};">
            <div class="compare-slot-header">
              <span class="compare-slot-tag" style="background: ${theme.bg}; color: ${theme.lightColor}; border: 1px solid ${theme.border};">
                ${theme.name}
              </span>
              <div class="compare-slot-actions">
                <button class="compare-btn-icon btn-change-slot" data-slot-index="${idx}" title="Change File">🔄</button>
                <button class="compare-btn-icon btn-remove-slot" data-slot-index="${idx}" title="Remove File">✕</button>
              </div>
            </div>
            <div class="compare-slot-filename" title="${s.path}">${fname}</div>
            <div class="compare-slot-meta">
              <span class="badge" style="font-size: 10px; padding: 1px 5px;">${fmt}</span>
              <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${summary || formatBytes(s.report.general?.file_size)}</span>
            </div>
          </div>
        `;
      }).join("")}

      ${slots.length < MAX_COMPARE_FILES ? `
        <div class="compare-slot-add-card" id="compare-add-slot-btn">
          <div style="font-size: 20px; margin-bottom: 4px;">➕</div>
          <div style="font-weight: 600; font-size: 12px; color: var(--text-main);">Add File (${slots.length + 1}/${MAX_COMPARE_FILES})</div>
          <div style="font-size: 11px; color: var(--text-muted);">Compare up to ${MAX_COMPARE_FILES} files</div>
        </div>
      ` : ""}
    </div>
  `;

  // Toolbar HTML
  const toolbarHtml = `
    <div class="compare-toolbar">
      <div class="compare-segmented-control">
        <button class="compare-filter-pill ${filter === "all" ? "active" : ""}" data-filter="all">
          All Fields <span class="compare-pill-count">${allRows.length}</span>
        </button>
        <button class="compare-filter-pill ${filter === "diff" ? "active" : ""}" data-filter="diff">
          Differences Only <span class="compare-pill-count diff">${diffRowsCount}</span>
        </button>
        <button class="compare-filter-pill ${filter === "identical" ? "active" : ""}" data-filter="identical">
          Identical Only <span class="compare-pill-count">${identicalRowsCount}</span>
        </button>
      </div>

      <div class="compare-search-box">
        <span style="font-size: 12px; opacity: 0.6;">🔍</span>
        <input type="text" id="compare-search-input" class="compare-search-input" placeholder="Search parameters..." value="${searchQuery}">
        ${searchQuery ? `<button id="btn-clear-compare-search" class="compare-btn-icon" style="font-size: 11px;">✕</button>` : ""}
      </div>

      <div class="compare-actions-group">
        <button id="btn-compare-export-csv" class="btn" title="Export comparison as CSV">📄 Export CSV</button>
        <button id="btn-compare-clear" class="btn" title="Clear all compared files">Clear</button>
      </div>
    </div>
  `;

  // Table HTML
  let tableRowsHtml = "";
  const catNames = Object.keys(categories);

  if (catNames.length === 0) {
    tableRowsHtml = `
      <tr>
        <td colspan="${slots.length + 3}" style="text-align: center; padding: 36px; color: var(--text-muted);">
          No fields matching the current filter <strong>"${filter}"</strong>${searchQuery ? ` and query "${searchQuery}"` : ""}.
        </td>
      </tr>
    `;
  } else {
    for (const cat of catNames) {
      tableRowsHtml += `
        <tr class="compare-category-header-row">
          <td colspan="${slots.length + 3}">${cat}</td>
        </tr>
      `;

      for (const row of categories[cat]) {
        const isDiffClass = row.isDiff ? "compare-row-diff" : "compare-row-identical";
        const statusBadge = row.isDiff
          ? `<span class="compare-badge diff">≠ Diff</span>`
          : `<span class="compare-badge identical">= Match</span>`;

        tableRowsHtml += `
          <tr class="${isDiffClass}">
            <td class="compare-td-cat" style="color: var(--text-dim); font-weight: 500; font-size: 11px;">${row.category}</td>
            <td class="compare-td-param" style="color: var(--text-main); font-weight: 600;">${row.field}</td>
            ${row.values.map((v, idx) => {
              const theme = SLOT_COLORS[idx] || SLOT_COLORS[0];
              const valueColor = row.isDiff ? theme.lightColor : "var(--text-main)";
              return `
                <td class="compare-td-val" style="font-family: var(--font-mono); font-size: 11.5px; color: ${valueColor}; word-break: break-word;">
                  ${v}
                </td>
              `;
            }).join("")}
            <td class="compare-td-status" style="text-align: center; width: 70px;">${statusBadge}</td>
          </tr>
        `;
      }
    }
  }

  const tableHtml = `
    <div class="compare-table-container">
      <table class="grid-table compare-matrix-table">
        <thead>
          <tr>
            <th style="width: 110px;">Category</th>
            <th style="width: 180px;">Parameter</th>
            ${slots.map((s, idx) => {
              const theme = SLOT_COLORS[idx] || SLOT_COLORS[0];
              return `
                <th style="color: ${theme.lightColor}; font-weight: 700;">
                  ${theme.name}: ${s.name || s.path.split("/").pop()}
                </th>
              `;
            }).join("")}
            <th style="width: 70px; text-align: center;">Status</th>
          </tr>
        </thead>
        <tbody>
          ${tableRowsHtml}
        </tbody>
      </table>
    </div>
  `;

  return `
    <div class="compare-view-wrapper">
      ${slotCardsHtml}
      ${toolbarHtml}
      ${tableHtml}
    </div>
  `;
}
