import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { MediaReport, ComparisonDiff } from "./types";
import { renderSummaryView } from "./views/summary";
import { renderTreeView } from "./views/tree";
import { renderGridView } from "./views/grid";
import { renderBitstreamView } from "./views/bitstream";
import { renderRawView } from "./views/raw";
import { renderDiffView } from "./views/diff";
import { renderBatchView } from "./views/batch";

export class MediaInfoApp {
  private activeTab: "summary" | "tree" | "grid" | "bitstream" | "raw" | "diff" | "batch" = "summary";
  private currentReport?: MediaReport;
  private currentRawFormat = "text";
  private rawContent = "";
  private batchReports: MediaReport[] = [];
  private currentDiff?: ComparisonDiff;
  private diffFileA?: string;
  private diffFileB?: string;
  private treeSearchQuery = "";
  private gridSearchQuery = "";

  constructor() {
    console.log("[app] constructor called");
    this.init();
  }

  private async init() {
    console.log("[app] init() starting");
    this.renderShell();
    this.setupListeners();
    this.setupShortcuts();
    this.setupDragDrop();
    await this.loadInitialFiles();
    console.log("[app] init() complete");
  }

  private async loadInitialFiles() {
    console.log("[app] loadInitialFiles() called");
    try {
      const files = await invoke<string[]>("get_initial_files");
      console.log("[app] got initial files:", files);
      if (files && files.length > 0) {
        if (files.length === 1) {
          await this.loadFile(files[0]);
        } else {
          await this.processBatch(files);
          this.activeTab = "batch";
          this.renderShell();
        }
      }
    } catch (err) {
      console.error("[app] loadInitialFiles error:", err);
      this.showError("Failed to load initial files: " + err);
    }
  }

  private showError(msg: string) {
    const el = document.getElementById("main-content-view");
    if (el) {
      el.innerHTML = `<div style="padding: 20px; color: #ef4444; font-family: var(--font-mono); background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3); border-radius: 8px; margin: 20px;">${msg}</div>` + el.innerHTML;
    }
  }

  private renderShell() {
    const container = document.getElementById("app-container");
    if (!container) {
      console.error("[app] #app-container not found!");
      return;
    }

    const fileName = this.currentReport?.general?.file_name
      || this.currentReport?.general?.file_path?.split("/").pop()
      || "No Media File Open";

    container.innerHTML = `
      <div class="titlebar" data-tauri-drag-region>
        <div class="titlebar-left" data-tauri-drag-region>
          <span class="app-logo-badge" data-tauri-drag-region>MEDIAINFO</span>
          <span class="titlebar-file-name" data-tauri-drag-region>${fileName}</span>
        </div>
        <div class="titlebar-actions">
          <button id="btn-open-file" class="btn btn-primary">📂 Open File</button>
          <button id="btn-open-folder" class="btn">📁 Open Folder</button>
        </div>
      </div>

      <div class="nav-tab-bar">
        <div class="tab-pill ${this.activeTab === "summary" ? "active" : ""}" data-tab="summary">Dashboard</div>
        <div class="tab-pill ${this.activeTab === "tree" ? "active" : ""}" data-tab="tree">Tree View</div>
        <div class="tab-pill ${this.activeTab === "grid" ? "active" : ""}" data-tab="grid">Data Grid</div>
        <div class="tab-pill ${this.activeTab === "bitstream" ? "active" : ""}" data-tab="bitstream">Bitstream</div>
        <div class="tab-pill ${this.activeTab === "raw" ? "active" : ""}" data-tab="raw">Raw Export</div>
        <div class="tab-pill ${this.activeTab === "diff" ? "active" : ""}" data-tab="diff">Compare</div>
        <div class="tab-pill ${this.activeTab === "batch" ? "active" : ""}" data-tab="batch">Batch (${this.batchReports.length})</div>
      </div>

      <div class="main-viewport" id="main-content-view">
        ${this.renderActiveView()}
      </div>
    `;

    this.attachViewEvents();
  }

  private renderActiveView(): string {
    if (this.activeTab === "diff") return renderDiffView(this.currentDiff);
    if (this.activeTab === "batch") return renderBatchView(this.batchReports);

    if (!this.currentReport) {
      return `
        <div class="empty-state-card" id="empty-dropzone">
          <div style="font-size: 44px; margin-bottom: 12px;">🎬</div>
          <div style="font-weight: 700; font-size: 16px; color: #fff;">Drop Media File or Folder Here</div>
          <div style="font-size: 12px; color: var(--text-muted); margin-top: 6px; max-width: 400px;">
            Instant zero-copy inspection for MP4, MKV, WebM, AVI, WAV, RF64, FLAC, MP3, AAC, MPEG-TS.
          </div>
          <div style="display: flex; gap: 12px; margin-top: 20px;">
            <button id="btn-empty-open-file" class="btn btn-primary">📂 Select Media File</button>
            <button id="btn-empty-open-folder" class="btn">📁 Select Directory</button>
          </div>
        </div>
      `;
    }

    switch (this.activeTab) {
      case "summary": return renderSummaryView(this.currentReport);
      case "tree": return renderTreeView(this.currentReport, this.treeSearchQuery);
      case "grid": return renderGridView(this.currentReport, this.gridSearchQuery);
      case "bitstream": return renderBitstreamView(this.currentReport);
      case "raw": return renderRawView(this.rawContent, this.currentRawFormat);
      default: return "";
    }
  }

  private setupListeners() {
    document.addEventListener("click", async (e) => {
      const target = e.target as HTMLElement;

      // Tab switching
      const tabEl = target.closest(".tab-pill") as HTMLElement | null;
      if (tabEl) {
        const tab = tabEl.getAttribute("data-tab") as any;
        if (tab && tab !== this.activeTab) {
          this.activeTab = tab;
          if (this.activeTab === "raw" && this.currentReport) {
            await this.loadRawContent(this.currentRawFormat);
          }
          this.renderShell();
        }
        return;
      }

      // Open file
      if (target.id === "btn-open-file" || target.id === "btn-empty-open-file") {
        console.log("[app] Open File button clicked");
        await this.openFileDialog();
        return;
      }

      // Open folder
      if (target.id === "btn-open-folder" || target.id === "btn-empty-open-folder") {
        console.log("[app] Open Folder button clicked");
        await this.openFolderDialog();
        return;
      }

      // Raw format switch
      if (target.closest(".raw-format-btn")) {
        const fmt = (target.closest(".raw-format-btn") as HTMLElement).getAttribute("data-format");
        if (fmt) {
          this.currentRawFormat = fmt;
          await this.loadRawContent(fmt);
          this.renderShell();
        }
        return;
      }

      // Copy raw
      if (target.id === "copy-raw-btn") {
        await navigator.clipboard.writeText(this.rawContent);
        target.innerText = "✅ Copied!";
        setTimeout(() => { target.innerText = "📋 Copy to Clipboard"; }, 1500);
        return;
      }

      // Diff select A/B
      if (target.id === "select-diff-a-btn") {
        const path = await this.pickSingleFile();
        if (path) {
          this.diffFileA = path;
          target.innerText = `A: ${path.split("/").pop()}`;
          if (this.diffFileA && this.diffFileB) await this.runDiff();
        }
        return;
      }
      if (target.id === "select-diff-b-btn") {
        const path = await this.pickSingleFile();
        if (path) {
          this.diffFileB = path;
          target.innerText = `B: ${path.split("/").pop()}`;
          if (this.diffFileA && this.diffFileB) await this.runDiff();
        }
        return;
      }

      // Batch row click
      const batchRow = target.closest(".batch-row") as HTMLElement | null;
      if (batchRow) {
        const idx = parseInt(batchRow.getAttribute("data-index") || "0", 10);
        if (this.batchReports[idx]) {
          this.currentReport = this.batchReports[idx];
          this.activeTab = "summary";
          this.renderShell();
        }
        return;
      }

      // Batch actions
      if (target.id === "batch-add-btn" || target.id === "batch-select-btn") {
        await this.openFileDialog();
        return;
      }
      if (target.id === "batch-clear-btn") {
        this.batchReports = [];
        this.renderShell();
        return;
      }
      if (target.id === "batch-export-csv-btn") {
        this.exportBatchCsv();
        return;
      }
    });
  }

  private attachViewEvents() {
    const treeInput = document.getElementById("tree-search-input") as HTMLInputElement | null;
    if (treeInput) {
      treeInput.addEventListener("input", () => {
        this.treeSearchQuery = treeInput.value;
        const main = document.getElementById("main-content-view");
        if (main && this.currentReport) main.innerHTML = renderTreeView(this.currentReport, this.treeSearchQuery);
      });
    }
    const gridInput = document.getElementById("grid-search-input") as HTMLInputElement | null;
    if (gridInput) {
      gridInput.addEventListener("input", () => {
        this.gridSearchQuery = gridInput.value;
        const main = document.getElementById("main-content-view");
        if (main && this.currentReport) main.innerHTML = renderGridView(this.currentReport, this.gridSearchQuery);
      });
    }
  }

  private setupShortcuts() {
    window.addEventListener("keydown", (e) => {
      const cmd = e.metaKey || e.ctrlKey;
      if (cmd && e.key === "o") { e.preventDefault(); this.openFileDialog(); }
      if (cmd && e.key >= "1" && e.key <= "7") {
        e.preventDefault();
        const tabs: any[] = ["summary", "tree", "grid", "bitstream", "raw", "diff", "batch"];
        this.activeTab = tabs[parseInt(e.key, 10) - 1];
        this.renderShell();
      }
    });
  }

  private async setupDragDrop() {
    try {
      const webview = getCurrentWebview();
      await webview.onDragDropEvent(async (event) => {
        if (event.payload.type === "drop" && event.payload.paths?.length) {
          const paths = event.payload.paths;
          console.log("[app] drag-drop:", paths);
          if (paths.length === 1) {
            await this.loadFile(paths[0]);
          } else {
            await this.processBatch(paths);
            this.activeTab = "batch";
            this.renderShell();
          }
        }
      });
      console.log("[app] drag-drop listener registered");
    } catch (err) {
      console.warn("[app] drag-drop setup failed (web preview mode?):", err);
    }
  }

  // --- File dialogs using Tauri dialog plugin ---
  private async openFileDialog() {
    console.log("[app] openFileDialog()");
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: "Media Files",
          extensions: ["mp4", "mkv", "mov", "avi", "wav", "flac", "mp3", "aac", "m4a", "webm", "ts", "ogg", "opus"],
        }],
      });
      console.log("[app] dialog result:", selected);
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length === 0) return;
      if (paths.length === 1) {
        await this.loadFile(paths[0]);
      } else {
        await this.processBatch(paths);
        this.activeTab = "batch";
        this.renderShell();
      }
    } catch (err) {
      console.error("[app] openFileDialog error:", err);
      this.showError("Open file dialog error: " + err);
    }
  }

  private async openFolderDialog() {
    console.log("[app] openFolderDialog()");
    try {
      const selected = await open({ directory: true, multiple: false });
      console.log("[app] folder dialog result:", selected);
      if (!selected || typeof selected !== "string") return;
      // TODO: Enumerate media files in directory via backend command
      // For now, treat the folder path as a single path (backend will handle it)
      this.showError("Folder batch scan not yet wired — please select individual files for now.");
    } catch (err) {
      console.error("[app] openFolderDialog error:", err);
      this.showError("Open folder dialog error: " + err);
    }
  }

  private async pickSingleFile(): Promise<string | null> {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: "Media Files",
          extensions: ["mp4", "mkv", "mov", "avi", "wav", "flac", "mp3", "aac", "m4a", "webm", "ts", "ogg", "opus"],
        }],
      });
      if (selected && typeof selected === "string") return selected;
      if (Array.isArray(selected) && selected.length > 0) return selected[0];
      return null;
    } catch {
      return null;
    }
  }

  // --- Core IPC ---
  async loadFile(path: string) {
    console.log("[app] loadFile()", path);
    try {
      const report = await invoke<MediaReport>("inspect_file", { path });
      console.log("[app] inspect_file result:", report?.general?.format);
      this.currentReport = report;
      this.activeTab = "summary";
      this.renderShell();
    } catch (err) {
      console.error("[app] loadFile error:", err);
      this.showError("Error inspecting " + path + ": " + err);
    }
  }

  private async loadRawContent(format: string) {
    if (!this.currentReport?.general?.file_path) return;
    try {
      this.rawContent = await invoke<string>("format_report", {
        path: this.currentReport.general.file_path,
        format,
      });
    } catch {
      this.rawContent = JSON.stringify(this.currentReport, null, 2);
    }
  }

  private async runDiff() {
    if (!this.diffFileA || !this.diffFileB) return;
    try {
      this.currentDiff = await invoke<ComparisonDiff>("compare_files", {
        pathA: this.diffFileA,
        pathB: this.diffFileB,
      });
      this.renderShell();
    } catch (err) {
      this.showError("Diff error: " + err);
    }
  }

  private async processBatch(paths: string[]) {
    console.log("[app] processBatch()", paths.length, "files");
    try {
      const reports = await invoke<MediaReport[]>("inspect_batch", { paths });
      this.batchReports = [...this.batchReports, ...reports];
      if (!this.currentReport && this.batchReports.length > 0) {
        this.currentReport = this.batchReports[0];
      }
      this.renderShell();
    } catch (err) {
      this.showError("Batch error: " + err);
    }
  }

  private exportBatchCsv() {
    if (this.batchReports.length === 0) return;
    const header = "File,Format,FileSize_Bytes,Duration_ms,Video_Codec,Resolution,Audio_Codec,Channels,SamplingRate\n";
    const rows = this.batchReports.map((r) => {
      const gen = r.general;
      const v = r.videos[0];
      const a = r.audios[0];
      return `"${gen.file_name || ""}",${gen.format},${gen.file_size},${gen.duration_ms || 0},${v ? v.format : ""},${v ? `${v.width}x${v.height}` : ""},${a ? a.format : ""},${a ? a.channels : ""},${a ? a.sampling_rate : ""}`;
    }).join("\n");
    const blob = new Blob([header + rows], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `mediainfo_batch_${Date.now()}.csv`;
    link.click();
  }
}
