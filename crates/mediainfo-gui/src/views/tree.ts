import { MediaReport } from "../types";

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function highlightMatch(text: string, query: string): string {
  if (!query) return escapeHtml(text);
  const escapedText = escapeHtml(text);
  const escapedQuery = escapeHtml(query);
  const regex = new RegExp(`(${escapedQuery.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
  return escapedText.replace(regex, `<mark class="tree-search-highlight">$1</mark>`);
}

export function renderTreeSections(report: MediaReport, searchQuery = ""): { html: string; matchCount: number } {
  const query = searchQuery.toLowerCase().trim();
  let totalMatches = 0;
  let sectionsHtml = "";

  // Helper to render key-value rows
  const renderRows = (obj: Record<string, any>) => {
    const entries = Object.entries(obj).filter(([k, v]) => {
      if (v === undefined || v === null || v === "") return false;
      if (k === "cover_data_base64" || k === "raw_attributes") return false;
      if (!query) return true;
      const kStr = k.toLowerCase();
      const vStr = typeof v === "object" ? JSON.stringify(v).toLowerCase() : String(v).toLowerCase();
      return kStr.includes(query) || vStr.includes(query);
    });

    if (query) {
      totalMatches += entries.length;
    }

    return entries
      .map(([k, v]) => {
        const valDisplay = typeof v === "object" ? JSON.stringify(v) : String(v);
        const highlightedKey = highlightMatch(k, query);
        const highlightedVal = highlightMatch(valDisplay, query);
        return `
          <div class="tree-key">${highlightedKey}</div>
          <div class="tree-val">${highlightedVal}</div>
        `;
      })
      .join("");
  };

  // 1. General Section
  const generalRows = renderRows(report.general);
  const generalMatches = (report.general && query) ? Object.entries(report.general).filter(([k, v]) => {
    if (v === undefined || v === null || v === "") return false;
    if (k === "cover_data_base64") return false;
    return k.toLowerCase().includes(query) || String(v).toLowerCase().includes(query);
  }).length : Object.keys(report.general || {}).length;

  if (generalRows || !query) {
    sectionsHtml += `
      <div class="tree-section" data-section-id="general">
        <div class="tree-header">
          <span>📁 General &bull; ${escapeHtml(report.general?.format || "Container")}</span>
          <span class="tree-badge">${generalMatches} fields</span>
        </div>
        <div class="tree-content">
          ${generalRows || '<div class="tree-empty-hint">No matching fields in General</div>'}
        </div>
      </div>
    `;
  }

  // 2. Video Tracks
  report.videos?.forEach((v, idx) => {
    const rows = renderRows(v);
    const count = Object.keys(v).length;
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section" data-section-id="video-${idx}">
          <div class="tree-header">
            <span>🎬 Video #${idx + 1} &bull; ${escapeHtml(v.format || "Video")} (${v.width || "?"}x${v.height || "?"})</span>
            <span class="tree-badge">${count} fields</span>
          </div>
          <div class="tree-content">
            ${rows || '<div class="tree-empty-hint">No matching fields in Video #' + (idx + 1) + '</div>'}
          </div>
        </div>
      `;
    }
  });

  // 3. Audio Tracks
  report.audios?.forEach((a, idx) => {
    const rows = renderRows(a);
    const count = Object.keys(a).length;
    const title = a.language ? `${a.format} (${a.language})` : `${a.format} ${a.channels || 2}ch`;
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section" data-section-id="audio-${idx}">
          <div class="tree-header">
            <span>🔊 Audio #${idx + 1} &bull; ${escapeHtml(title)}</span>
            <span class="tree-badge">${count} fields</span>
          </div>
          <div class="tree-content">
            ${rows || '<div class="tree-empty-hint">No matching fields in Audio #' + (idx + 1) + '</div>'}
          </div>
        </div>
      `;
    }
  });

  // 4. Subtitle / Text Tracks
  report.texts?.forEach((t, idx) => {
    const rows = renderRows(t);
    const count = Object.keys(t).length;
    const title = t.language ? `${t.format} (${t.language})` : t.format;
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section" data-section-id="text-${idx}">
          <div class="tree-header">
            <span>💬 Subtitle #${idx + 1} &bull; ${escapeHtml(title)}</span>
            <span class="tree-badge">${count} fields</span>
          </div>
          <div class="tree-content">
            ${rows || '<div class="tree-empty-hint">No matching fields in Subtitle #' + (idx + 1) + '</div>'}
          </div>
        </div>
      `;
    }
  });

  // 5. Chapters / Menu
  if (report.menu && report.menu.chapters && report.menu.chapters.length > 0) {
    const chapterEntries = report.menu.chapters.filter((c) => {
      if (!query) return true;
      const title = (c.title || "").toLowerCase();
      const timeStr = new Date(c.timestamp_ms).toISOString().slice(11, 23).toLowerCase();
      return title.includes(query) || timeStr.includes(query);
    });

    if (query) {
      totalMatches += chapterEntries.length;
    }

    const chapterRows = chapterEntries
      .map((c) => {
        const sec = Math.floor(c.timestamp_ms / 1000);
        const timeStr = new Date(c.timestamp_ms).toISOString().slice(11, 23);
        const title = c.title || `Chapter @ ${sec}s`;
        return `
          <div class="tree-key">${highlightMatch(timeStr, query)}</div>
          <div class="tree-val">${highlightMatch(title, query)}</div>
        `;
      })
      .join("");

    if (chapterRows || !query) {
      sectionsHtml += `
        <div class="tree-section" data-section-id="chapters">
          <div class="tree-header">
            <span>📑 Chapters &bull; ${report.menu.chapters.length} items</span>
            <span class="tree-badge">${chapterEntries.length} items</span>
          </div>
          <div class="tree-content">
            ${chapterRows || '<div class="tree-empty-hint">No matching chapters</div>'}
          </div>
        </div>
      `;
    }
  }

  if (!sectionsHtml) {
    sectionsHtml = `
      <div class="tree-empty-state">
        <div style="font-size: 28px; margin-bottom: 8px;">🔍</div>
        <div style="font-weight: 600; color: #fff;">No metadata fields matching "${escapeHtml(searchQuery)}"</div>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">Try searching for Codec, BitRate, Resolution, Format, ColorSpace, or Duration</div>
        <button id="btn-clear-tree-search" class="btn btn-primary" style="margin-top: 14px; font-size: 12px; padding: 6px 14px;">Clear Search Filter</button>
      </div>
    `;
  }

  return { html: sectionsHtml, matchCount: totalMatches };
}

export function renderTreeView(report: MediaReport, searchQuery = ""): string {
  const { html, matchCount } = renderTreeSections(report, searchQuery);
  const badgeText = searchQuery.trim() ? `${matchCount} matches` : "Live Filter";

  return `
    <div class="tree-view-wrapper">
      <div class="tree-toolbar">
        <div class="tree-search-box">
          <span class="tree-search-icon">🔍</span>
          <input 
            id="tree-search-input" 
            type="text" 
            placeholder="Instant filter fields (e.g. BitDepth, CodecID, MaxCLL, Duration, Channels)..." 
            value="${escapeHtml(searchQuery)}" 
            autocomplete="off"
            spellcheck="false"
          />
          ${searchQuery ? '<button id="btn-tree-search-clear" class="tree-search-clear-btn" title="Clear filter">✕</button>' : ""}
        </div>
        <div id="tree-search-badge" class="tree-status-badge ${searchQuery.trim() ? "active" : ""}">
          ${badgeText}
        </div>
      </div>
      <div id="tree-sections-container">
        ${html}
      </div>
    </div>
  `;
}
