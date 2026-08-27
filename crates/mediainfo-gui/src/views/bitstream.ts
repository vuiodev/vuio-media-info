import { BitstreamNode, MediaReport } from "../types";

export function renderBitstreamView(report: MediaReport): string {
  const root = report.bitstream_root;

  if (!root) {
    return `
      <div class="empty-state-card">
        <div style="font-size: 32px; margin-bottom: 12px;">📦</div>
        <div style="font-weight: 600; font-size: 14px;">No Bitstream Tree Available</div>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
          Container box hierarchy is generated for MP4/ISOBMFF and Matroska/WebM EBML files.
        </div>
      </div>
    `;
  }

  const renderNode = (node: BitstreamNode, depth = 0): string => {
    const indent = depth * 20;
    const isLeaf = !node.children || node.children.length === 0;

    const formatOffset = `0x${node.offset.toString(16).toUpperCase().padStart(8, "0")}`;
    const formatSize = `${node.size.toLocaleString()} bytes`;

    let html = `
      <div style="display: flex; align-items: center; justify-content: space-between; padding: 6px 12px; margin-left: ${indent}px; border-bottom: 1px solid var(--border-subtle); background: ${depth % 2 === 0 ? "rgba(255,255,255,0.01)" : "transparent"};">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="color: ${isLeaf ? "var(--text-dim)" : "var(--accent-blue)"}; font-size: 11px;">${isLeaf ? "▫" : "▾"}</span>
          <span style="font-weight: 600; font-family: var(--font-mono); color: var(--text-main);">${node.name}</span>
          ${node.description ? `<span style="color: var(--text-muted); font-size: 11px;">(${node.description})</span>` : ""}
        </div>
        <div style="display: flex; align-items: center; gap: 16px; font-family: var(--font-mono); font-size: 11px;">
          <span style="color: var(--text-dim);">${formatOffset}</span>
          <span style="color: var(--accent-amber);">${formatSize}</span>
        </div>
      </div>
    `;

    if (node.children && node.children.length > 0) {
      html += node.children.map((c) => renderNode(c, depth + 1)).join("");
    }

    return html;
  };

  return `
    <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden;">
      <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; background: rgba(0,0,0,0.3); border-bottom: 1px solid var(--border-subtle); font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--text-muted);">
        <span>Element / Box Hierarchy</span>
        <div style="display: flex; gap: 24px;">
          <span>Offset</span>
          <span>Size</span>
        </div>
      </div>
      <div>
        ${renderNode(root)}
      </div>
    </div>
  `;
}
