import "./styles/native.css";
import { MediaInfoApp } from "./app";

console.log("[mediainfo-gui] main.ts loaded");

window.addEventListener("DOMContentLoaded", () => {
  console.log("[mediainfo-gui] DOMContentLoaded fired");
  try {
    const app = new MediaInfoApp();
    console.log("[mediainfo-gui] MediaInfoApp created successfully", app);
  } catch (err) {
    console.error("[mediainfo-gui] FATAL: Failed to create MediaInfoApp:", err);
    document.getElementById("app-container")!.innerHTML = `
      <div style="padding: 40px; color: red; font-family: monospace;">
        <h2>MediaInfo GUI Init Error</h2>
        <pre>${err}</pre>
      </div>
    `;
  }
});
