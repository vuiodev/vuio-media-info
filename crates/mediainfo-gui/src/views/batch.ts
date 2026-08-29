import { MediaReport } from "../types";

export function renderBatchView(reports: MediaReport[], currentIndex = 0): string {
  if (reports.length === 0) {
    return `
      <div class="empty-state-card" id="batch-drop-card">
        <div style="font-size: 32px; margin-bottom: 12px;">📁</div>
        <div style="font-weight: 600; font-size: 14px;">Multi-File Batch Queue</div>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px; margin-bottom: 16px;">
          Drag and drop multiple media files or directories to batch inspect them concurrently in milliseconds.
        </div>
        <button id="batch-select-btn" class="btn btn-primary">Select Files / Folder</button>
      </div>
    `;
  }

  const formatSize = (bytes: number) => {
    if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
    if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  };

  const formatDuration = (ms?: number) => {
    if (!ms || ms <= 0) return "-";
    const totalMs = Math.round(ms);
    const h = Math.floor(totalMs / 3600000);
    const m = Math.floor((totalMs % 3600000) / 60000);
    const sec = Math.floor((totalMs % 60000) / 1000);
    const millis = totalMs % 1000;

    if (h > 0) {
      return `${h}h ${m}m ${sec}s ${millis}ms`;
    } else if (m > 0) {
      return `${m}m ${sec}s ${millis}ms`;
    } else if (sec > 0) {
      return `${sec}s ${millis}ms`;
    } else {
      return `${millis}ms`;
    }
  };

  return `
    <div style="display: flex; flex-direction: column; gap: 12px;">
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <div style="font-weight: 600; font-size: 13px;">
          Batch Queue (${reports.length} files scanned)
        </div>
        <div style="display: flex; gap: 6px;">
          <button id="batch-add-btn" class="btn">➕ Add Files</button>
          <button id="batch-add-folder-btn" class="btn">📁 Add Folder</button>
          <button id="batch-export-csv-btn" class="btn btn-primary">📊 Export CSV</button>
          <button id="batch-clear-btn" class="btn">🗑️ Clear</button>
        </div>
      </div>

      <table class="grid-table">
        <thead>
          <tr>
            <th style="width: 36px;">#</th>
            <th>File Name</th>
            <th style="width: 90px;">Format</th>
            <th style="width: 85px;">Size</th>
            <th style="width: 105px;">Duration</th>
            <th style="width: 130px;">Video</th>
            <th style="width: 130px;">Audio</th>
          </tr>
        </thead>
        <tbody>
          ${reports
            .map((r, idx) => {
              const gen = r.general;
              const v = r.videos[0];
              const a = r.audios[0];
              const name = gen.file_name || gen.file_path?.split("/").pop() || `File #${idx + 1}`;
              const isActive = idx === currentIndex;

              return `
              <tr class="batch-row${isActive ? " batch-row-active" : ""}" data-index="${idx}" style="cursor: pointer; ${isActive ? "background: rgba(59, 130, 246, 0.15); font-weight: 600;" : ""}">
                <td style="color: var(--text-dim); font-family: var(--font-mono); font-size: 10px;">${idx + 1}</td>
                <td style="color: ${isActive ? "#fff" : "var(--text-main)"}; word-break: break-all;">${name}</td>
                <td style="color: var(--accent-blue); font-weight: 500;">${gen.format}</td>
                <td style="color: var(--text-muted); font-family: var(--font-mono); font-size: 11px;">${formatSize(gen.file_size)}</td>
                <td style="color: var(--text-muted); font-family: var(--font-mono); font-size: 11px;">${formatDuration(gen.duration_ms)}</td>
                <td style="color: #60a5fa; font-size: 11px;">${v ? `${v.format} ${v.width}x${v.height}` : "-"}</td>
                <td style="color: #34d399; font-size: 11px;">${a ? `${a.format} ${a.channels}ch` : "-"}</td>
              </tr>
            `;
            })
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}
