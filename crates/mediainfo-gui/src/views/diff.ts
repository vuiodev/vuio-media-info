import { ComparisonDiff } from "../types";

export function renderDiffView(diff?: ComparisonDiff): string {
  if (!diff) {
    return `
      <div class="empty-state-card" id="diff-open-card">
        <div style="font-size: 32px; margin-bottom: 12px;">⚖️</div>
        <div style="font-weight: 600; font-size: 14px;">Compare Two Media Files</div>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px; margin-bottom: 16px;">
          Select two files to visually inspect technical differences in codecs, bitrates, resolutions, audio tracks, and colorimetry.
        </div>
        <div style="display: flex; gap: 12px;">
          <button id="select-diff-a-btn" class="btn btn-primary">Select File A</button>
          <button id="select-diff-b-btn" class="btn btn-primary">Select File B</button>
        </div>
      </div>
    `;
  }

  const fnameA = diff.file_a.split("/").pop() || diff.file_a;
  const fnameB = diff.file_b.split("/").pop() || diff.file_b;

  return `
    <div style="display: flex; flex-direction: column; gap: 16px;">
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
        <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 12px 16px;">
          <div style="font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--accent-blue);">File A</div>
          <div style="font-weight: 600; font-size: 13px; margin-top: 4px; word-break: break-all;">${fnameA}</div>
        </div>
        <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 12px 16px;">
          <div style="font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--accent-purple);">File B</div>
          <div style="font-weight: 600; font-size: 13px; margin-top: 4px; word-break: break-all;">${fnameB}</div>
        </div>
      </div>

      <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden;">
        <div style="padding: 10px 16px; background: rgba(0,0,0,0.3); border-bottom: 1px solid var(--border-subtle); font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--text-muted);">
          Key Parameter Differences (${diff.differences.length})
        </div>
        <table class="grid-table" style="border: none;">
          <thead>
            <tr>
              <th style="width: 120px;">Category</th>
              <th style="width: 180px;">Parameter</th>
              <th>Value in File A</th>
              <th>Value in File B</th>
            </tr>
          </thead>
          <tbody>
            ${
              diff.differences.length > 0
                ? diff.differences
                    .map(
                      (d) => `
                <tr>
                  <td style="font-weight: 600; color: var(--text-dim);">${d.category}</td>
                  <td style="color: var(--text-main); font-weight: 500;">${d.field}</td>
                  <td style="color: #60a5fa; font-family: var(--font-mono); font-size: 11px;">${d.value_a}</td>
                  <td style="color: #c084fc; font-family: var(--font-mono); font-size: 11px;">${d.value_b}</td>
                </tr>
              `
                    )
                    .join("")
                : `<tr><td colspan="4" style="text-align: center; color: var(--accent-emerald); padding: 24px; font-weight: 600;">✨ All key parameters are identical!</td></tr>`
            }
          </tbody>
        </table>
      </div>
    </div>
  `;
}
