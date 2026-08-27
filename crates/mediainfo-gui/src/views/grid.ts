import { MediaReport } from "../types";

export function renderGridView(report: MediaReport, searchQuery = ""): string {
  const query = searchQuery.toLowerCase().trim();

  interface RowData {
    stream: string;
    key: string;
    val: string;
  }

  const rows: RowData[] = [];

  const extract = (stream: string, obj: Record<string, any>) => {
    Object.entries(obj).forEach(([k, v]) => {
      if (v !== undefined && v !== null && v !== "" && k !== "cover_data_base64") {
        rows.push({ stream, key: k, val: String(v) });
      }
    });
  };

  extract("General", report.general);
  report.videos.forEach((v, i) => extract(`Video #${i + 1}`, v));
  report.audios.forEach((a, i) => extract(`Audio #${i + 1}`, a));
  report.texts.forEach((t, i) => extract(`Text #${i + 1}`, t));

  const filtered = rows.filter(
    (r) =>
      !query ||
      r.stream.toLowerCase().includes(query) ||
      r.key.toLowerCase().includes(query) ||
      r.val.toLowerCase().includes(query)
  );

  return `
    <div style="display: flex; flex-direction: column; gap: 12px;">
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <input 
          id="grid-search-input" 
          type="text" 
          placeholder="Filter data grid parameters..." 
          value="${searchQuery}" 
          style="width: 100%; max-width: 400px; padding: 8px 12px; background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 6px; color: #fff; font-size: 12px;"
        />
        <div style="font-size: 11px; color: var(--text-muted); font-weight: 500;">
          Showing ${filtered.length} of ${rows.length} parameters
        </div>
      </div>

      <table class="grid-table">
        <thead>
          <tr>
            <th style="width: 140px;">Stream</th>
            <th style="width: 240px;">Parameter</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          ${
            filtered.length > 0
              ? filtered
                  .map(
                    (r) => `
              <tr>
                <td style="font-weight: 600; color: var(--accent-blue);">${r.stream}</td>
                <td style="color: var(--text-muted); font-family: var(--font-mono); font-size: 11px;">${r.key}</td>
                <td style="color: var(--text-main); font-family: var(--font-mono); font-size: 11px; word-break: break-all; user-select: text; -webkit-user-select: text;">${r.val}</td>
              </tr>
            `
                  )
                  .join("")
              : `<tr><td colspan="3" style="text-align: center; color: var(--text-dim); padding: 30px;">No matching parameters</td></tr>`
          }
        </tbody>
      </table>
    </div>
  `;
}
