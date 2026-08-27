export function renderRawView(content: string, format = "text"): string {
  const formats = ["text", "json", "xml", "csv", "html"];

  return `
    <div style="display: flex; flex-direction: column; gap: 12px;">
      <div style="display: flex; align-items: center; justify-content: space-between;">
        <div style="display: flex; gap: 6px;">
          ${formats
            .map(
              (f) => `
            <button class="btn raw-format-btn ${f === format ? "btn-primary" : ""}" data-format="${f}">
              ${f.toUpperCase()}
            </button>
          `
            )
            .join("")}
        </div>
        <button id="copy-raw-btn" class="btn">
          📋 Copy to Clipboard
        </button>
      </div>

      <pre class="raw-codeblock"><code>${escapeHtml(content)}</code></pre>
    </div>
  `;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
