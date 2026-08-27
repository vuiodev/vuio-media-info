import { MediaReport } from "../types";

export function renderTreeView(report: MediaReport, searchQuery = ""): string {
  const query = searchQuery.toLowerCase().trim();

  let sectionsHtml = "";

  // Helper to render key-value rows
  const renderRows = (obj: Record<string, any>) => {
    return Object.entries(obj)
      .filter(([k, v]) => {
        if (v === undefined || v === null || v === "") return false;
        if (k === "cover_data_base64") return false;
        if (!query) return true;
        return k.toLowerCase().includes(query) || String(v).toLowerCase().includes(query);
      })
      .map(
        ([k, v]) => `
        <div class="tree-key">${k}</div>
        <div class="tree-val">${v}</div>
      `
      )
      .join("");
  };

  // 1. General
  const generalRows = renderRows(report.general);
  if (generalRows || !query) {
    sectionsHtml += `
      <div class="tree-section">
        <div class="tree-header">📁 General (${report.general.format})</div>
        <div class="tree-content">${generalRows || '<div style="color: var(--text-dim);">No matching fields</div>'}</div>
      </div>
    `;
  }

  // 2. Videos
  report.videos.forEach((v, idx) => {
    const rows = renderRows(v);
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section">
          <div class="tree-header">🎬 Video #${idx + 1} (${v.format} - ${v.width}x${v.height})</div>
          <div class="tree-content">${rows || '<div style="color: var(--text-dim);">No matching fields</div>'}</div>
        </div>
      `;
    }
  });

  // 3. Audios
  report.audios.forEach((a, idx) => {
    const rows = renderRows(a);
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section">
          <div class="tree-header">🔊 Audio #${idx + 1} (${a.format} ${a.channels}ch - ${a.language || a.title || "Track"})</div>
          <div class="tree-content">${rows || '<div style="color: var(--text-dim);">No matching fields</div>'}</div>
        </div>
      `;
    }
  });

  // 4. Texts
  report.texts.forEach((t, idx) => {
    const rows = renderRows(t);
    if (rows || !query) {
      sectionsHtml += `
        <div class="tree-section">
          <div class="tree-header">💬 Text / Subtitle #${idx + 1} (${t.format} - ${t.language || t.title || "Subtitle"})</div>
          <div class="tree-content">${rows || '<div style="color: var(--text-dim);">No matching fields</div>'}</div>
        </div>
      `;
    }
  });

  // 5. Chapters
  if (report.menu && report.menu.chapters.length > 0) {
    const chapterRows = report.menu.chapters
      .map((c) => {
        const sec = Math.floor(c.timestamp_ms / 1000);
        const timeStr = new Date(c.timestamp_ms).toISOString().slice(11, 23);
        return `
          <div class="tree-key">${timeStr}</div>
          <div class="tree-val">${c.title || `Chapter @ ${sec}s`}</div>
        `;
      })
      .join("");

    sectionsHtml += `
      <div class="tree-section">
        <div class="tree-header">📑 Chapters (${report.menu.chapters.length})</div>
        <div class="tree-content">${chapterRows}</div>
      </div>
    `;
  }

  return `
    <div style="display: flex; flex-direction: column; gap: 12px;">
      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
        <input 
          id="tree-search-input" 
          type="text" 
          placeholder="Filter fields (e.g. BitDepth, CodecID, MaxCLL, Duration)..." 
          value="${searchQuery}" 
          style="width: 100%; max-width: 400px; padding: 8px 12px; background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 6px; color: #fff; font-size: 12px;"
        />
      </div>
      ${sectionsHtml}
    </div>
  `;
}
