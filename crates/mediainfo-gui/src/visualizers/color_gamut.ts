import { VideoTrack } from "../types";

export function renderColorGamutVisualizer(track?: VideoTrack): string {
  if (!track) return "";

  const primaries = track.color_primaries || "BT.709 / sRGB";
  const matrix = track.matrix_coefficients || "BT.709";
  const transfer = track.transfer_characteristics || "BT.709";
  const bitDepth = track.bit_depth ? `${track.bit_depth}-bit` : "8-bit";

  const isHdr =
    primaries.includes("2020") ||
    transfer.includes("PQ") ||
    transfer.includes("SMPTE ST 2084") ||
    transfer.includes("HLG") ||
    track.hdr_format !== undefined;

  const hdrBadge = isHdr
    ? `<span class="badge badge-hdr">HDR Wide Color Gamut</span>`
    : `<span class="badge badge-video">SDR Standard Gamut</span>`;

  return `
    <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 12px; padding: 20px; margin-top: 16px;">
      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
        <div style="font-weight: 600; font-size: 14px;">🎨 Colorimetry & Gamut Space: <span style="color: var(--accent-amber);">${primaries}</span></div>
        ${hdrBadge}
      </div>
      <div style="display: grid; grid-template-columns: 180px 1fr; gap: 20px; align-items: center;">
        <svg viewBox="0 0 200 200" style="width: 100%; max-width: 180px; background: rgba(0,0,0,0.3); border-radius: 8px; padding: 8px;">
          <!-- CIE 1931 Spectrum Outer Tongue Outline -->
          <path d="M 30 170 Q 20 120 40 60 Q 70 20 130 30 Q 180 60 175 140 Z" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="1.5"/>
          <!-- Rec 2020 Triangle -->
          <polygon points="160,135 125,40 38,160" fill="rgba(245, 158, 11, 0.15)" stroke="#f59e0b" stroke-width="1.5"/>
          <!-- DCI-P3 Triangle -->
          <polygon points="150,135 115,55 45,155" fill="rgba(139, 92, 246, 0.15)" stroke="#8b5cf6" stroke-width="1.5" stroke-dasharray="2,2"/>
          <!-- Rec 709 Triangle -->
          <polygon points="140,135 105,70 50,150" fill="rgba(59, 130, 246, 0.2)" stroke="#3b82f6" stroke-width="1.5"/>
          <!-- Axis Points -->
          <circle cx="140" cy="135" r="3" fill="#ef4444"/>
          <circle cx="105" cy="70" r="3" fill="#22c55e"/>
          <circle cx="50" cy="150" r="3" fill="#3b82f6"/>
        </svg>

        <div style="display: grid; grid-template-columns: 140px 1fr; row-gap: 8px; font-size: 12px;">
          <span style="color: var(--text-muted);">Color Primaries:</span>
          <span style="color: var(--text-main); font-weight: 500;">${primaries}</span>

          <span style="color: var(--text-muted);">Transfer Function:</span>
          <span style="color: var(--text-main); font-weight: 500;">${transfer}</span>

          <span style="color: var(--text-muted);">Matrix Coefficients:</span>
          <span style="color: var(--text-main); font-weight: 500;">${matrix}</span>

          <span style="color: var(--text-muted);">Color Depth:</span>
          <span style="color: var(--text-main); font-weight: 500;">${bitDepth} (${track.chroma_subsampling || "4:2:0"})</span>

          ${
            track.maximum_content_light_level
              ? `
            <span style="color: var(--text-muted);">MaxCLL / MaxFALL:</span>
            <span style="color: var(--accent-amber); font-weight: 600;">${track.maximum_content_light_level} cd/m² / ${track.maximum_frame_average_light_level || 0} cd/m²</span>
          `
              : ""
          }
        </div>
      </div>
    </div>
  `;
}
