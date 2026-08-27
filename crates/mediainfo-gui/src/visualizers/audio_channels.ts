import { AudioTrack } from "../types";

export function renderAudioChannelVisualizer(track?: AudioTrack): string {
  if (!track) return `<div style="color: var(--text-dim);">No audio track selected</div>`;

  const channels = track.channels;
  const layout = track.channel_layout || `${channels} Channels`;

  let speakers: { label: string; desc: string; pos: string }[] = [];

  if (channels === 1) {
    speakers = [{ label: "C", desc: "Center", pos: "top" }];
  } else if (channels === 2) {
    speakers = [
      { label: "L", desc: "Front Left", pos: "left" },
      { label: "R", desc: "Front Right", pos: "right" },
    ];
  } else if (channels === 6) {
    speakers = [
      { label: "L", desc: "Front Left", pos: "top-left" },
      { label: "C", desc: "Center", pos: "top" },
      { label: "R", desc: "Front Right", pos: "top-right" },
      { label: "LFE", desc: "Subwoofer", pos: "center" },
      { label: "Ls", desc: "Surround Left", pos: "bottom-left" },
      { label: "Rs", desc: "Surround Right", pos: "bottom-right" },
    ];
  } else if (channels >= 8) {
    speakers = [
      { label: "L", desc: "Front Left", pos: "top-left" },
      { label: "C", desc: "Center", pos: "top" },
      { label: "R", desc: "Front Right", pos: "top-right" },
      { label: "LFE", desc: "Subwoofer", pos: "center" },
      { label: "Ls", desc: "Side Left", pos: "middle-left" },
      { label: "Rs", desc: "Side Right", pos: "middle-right" },
      { label: "Lsr", desc: "Rear Left", pos: "bottom-left" },
      { label: "Rsr", desc: "Rear Right", pos: "bottom-right" },
    ];
  } else {
    for (let i = 1; i <= channels; i++) {
      speakers.push({ label: `CH${i}`, desc: `Channel ${i}`, pos: "row" });
    }
  }

  const atmosBadge = track.dolby_atmos_present
    ? `<span class="badge badge-atmos">Dolby Atmos</span>`
    : "";

  return `
    <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 12px; padding: 20px; margin-top: 16px;">
      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
        <div style="font-weight: 600; font-size: 14px;">🔊 Audio Spatial Layout: <span style="color: var(--accent-blue);">${layout}</span></div>
        ${atmosBadge}
      </div>
      <div class="channel-speaker-grid">
        ${speakers
          .map(
            (s) => `
          <div class="speaker-node">
            <div class="speaker-icon">${s.label}</div>
            <div style="font-size: 10px; color: var(--text-muted); font-weight: 500;">${s.desc}</div>
          </div>
        `
          )
          .join("")}
      </div>
    </div>
  `;
}
