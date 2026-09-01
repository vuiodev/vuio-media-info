import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { AppSettings, MediaReport } from "./types";
import { renderSummaryView } from "./views/summary";
import { renderTreeView, renderTreeSections } from "./views/tree";
import { renderRawView } from "./views/raw";
import { renderDiffView, CompareSlot, CompareFilter, buildComparisonRows } from "./views/diff";
import { renderBatchView } from "./views/batch";

export class MediaInfoApp {
  private activeTab: "summary" | "tree" | "raw" | "diff" | "batch" = "summary";
  private currentReport?: MediaReport;
  private currentRawFormat = "text";
  private rawContent = "";
  private batchReports: MediaReport[] = [];
  private currentBatchIndex = 0;
  private supportedExtensions: string[] = [];
  private treeSearchQuery = "";
  private settings: AppSettings = { remember_window_state: true, window_maximized: false };
  private isSettingsOpen = false;
  private appVersion = "0.1.4";

  // Multi-file comparison state (up to 4 files max)
  private compareSlots: CompareSlot[] = [];
  private compareFilter: CompareFilter = "all";
  private compareSearchQuery = "";

  constructor() {
    console.log("[app] constructor called");
    this.init();
  }

  private async init() {
    console.log("[app] init() starting");
    try {
      this.supportedExtensions = await invoke<string[]>("get_supported_extensions");
      console.log("[app] supported extensions loaded:", this.supportedExtensions.length);
    } catch (e) {
      console.warn("[app] could not fetch supported extensions:", e);
    }
    try {
      this.settings = await invoke<AppSettings>("get_app_settings");
      console.log("[app] settings loaded:", this.settings);
    } catch (e) {
      console.warn("[app] could not fetch settings:", e);
    }
    try {
      const info = await invoke<{ version: string }>("get_app_info");
      if (info?.version) this.appVersion = info.version;
    } catch (e) {
      console.warn("[app] could not fetch app info:", e);
    }
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
          this.currentBatchIndex = 0;
          this.currentReport = this.batchReports[0];
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
      el.innerHTML = `<div style="padding: 16px; color: #ef4444; font-family: var(--font-mono); background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3); border-radius: 8px; margin-bottom: 16px;">${msg}</div>` + el.innerHTML;
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

    const quickSwitcherHtml = this.batchReports.length > 1 ? `
      <div class="quick-switcher-bar">
        <div class="quick-switcher-left">
          <button id="btn-prev-file" class="btn btn-stepper" ${this.currentBatchIndex === 0 ? "disabled" : ""} title="Previous File ([ or Alt+Left])">◀ Prev</button>
          <select id="quick-file-select" class="quick-select-dropdown">
            ${this.batchReports.map((r, i) => {
              const name = r.general?.file_name || r.general?.file_path?.split("/").pop() || `File #${i + 1}`;
              const fmt = r.general?.format || "";
              return `<option value="${i}" ${i === this.currentBatchIndex ? "selected" : ""}>${i + 1}. ${name} (${fmt})</option>`;
            }).join("")}
          </select>
          <button id="btn-next-file" class="btn btn-stepper" ${this.currentBatchIndex >= this.batchReports.length - 1 ? "disabled" : ""} title="Next File (] or Alt+Right)">Next ▶</button>
        </div>
        <div class="quick-switcher-right">
          <span class="batch-counter-badge">${this.currentBatchIndex + 1} / ${this.batchReports.length}</span>
        </div>
      </div>
    ` : "";

    const settingsModalHtml = this.isSettingsOpen ? `
      <div class="modal-backdrop" id="settings-modal-backdrop">
        <div class="modal-card">
          <div class="modal-header">
            <div class="modal-title">⚙️ Preferences</div>
            <button class="modal-close-btn" id="btn-close-settings" title="Close">✕</button>
          </div>
          <div class="modal-body">
            <div class="settings-section">
              <div class="settings-section-title">Window & Interface</div>
              <div class="settings-item">
                <div class="settings-item-info">
                  <div class="settings-item-label">Remember window size & position</div>
                  <div class="settings-item-desc">Automatically restore dimensions and screen placement on launch</div>
                </div>
                <label class="switch">
                  <input type="checkbox" id="toggle-remember-window" ${this.settings.remember_window_state ? "checked" : ""}>
                  <span class="slider"></span>
                </label>
              </div>
              <div class="settings-item">
                <div class="settings-item-info">
                  <div class="settings-item-label">Reset Window Dimensions</div>
                  <div class="settings-item-desc">Restore default window size (820 × 560) and center on screen</div>
                </div>
                <button id="btn-reset-window" class="btn">Reset</button>
              </div>
            </div>

            <div class="settings-section">
              <div class="settings-section-title">Engine Information</div>
              <div class="settings-item">
                <div class="settings-item-info">
                  <div class="settings-item-label">VuIO Media Info</div>
                  <div class="settings-item-desc">v${this.appVersion} &bull; Pure Rust 2024 engine (zero FFmpeg/libmediainfo runtime)</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    ` : "";

    container.innerHTML = `
      <div class="titlebar" data-tauri-drag-region>
        <div class="titlebar-left" data-tauri-drag-region>
          <span class="app-logo-badge" data-tauri-drag-region>MEDIAINFO</span>
          <span class="titlebar-file-name" data-tauri-drag-region>${fileName}</span>
        </div>
        <div class="titlebar-actions">
          <button id="btn-open-file" class="btn btn-primary">📂 Open File</button>
          <button id="btn-open-folder" class="btn">📁 Open Folder</button>
          <button id="btn-open-settings" class="btn" title="Preferences (Cmd+, / Ctrl+,)">⚙️</button>
        </div>
      </div>

      <div class="nav-tab-bar">
        <div class="tab-pill ${this.activeTab === "summary" ? "active" : ""}" data-tab="summary">Dashboard</div>
        <div class="tab-pill ${this.activeTab === "tree" ? "active" : ""}" data-tab="tree">Tree View</div>
        <div class="tab-pill ${this.activeTab === "raw" ? "active" : ""}" data-tab="raw">Raw Export</div>
        <div class="tab-pill ${this.activeTab === "diff" ? "active" : ""}" data-tab="diff">Compare (${this.compareSlots.length})</div>
        <div class="tab-pill ${this.activeTab === "batch" ? "active" : ""}" data-tab="batch">Batch (${this.batchReports.length})</div>
      </div>

      ${quickSwitcherHtml}

      <div class="main-viewport" id="main-content-view">
        ${this.renderActiveView()}
      </div>

      ${settingsModalHtml}
    `;

    this.attachViewEvents();
  }

  private renderActiveView(): string {
    if (this.activeTab === "diff") {
      // If slots are empty and current report exists, pre-seed slot 1 for seamless UX
      if (this.compareSlots.length === 0 && this.currentReport && this.currentReport.general?.file_path) {
        const path = this.currentReport.general.file_path;
        const name = this.currentReport.general.file_name || path.split("/").pop() || "Media File";
        this.compareSlots.push({ path, name, report: this.currentReport });
      }
      return renderDiffView(this.compareSlots, this.compareFilter, this.compareSearchQuery, this.batchReports);
    }

    if (this.activeTab === "batch") return renderBatchView(this.batchReports, this.currentBatchIndex);

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
      case "raw": return renderRawView(this.rawContent, this.currentRawFormat);
      default: return "";
    }
  }

  private async switchBatchFile(index: number) {
    if (index < 0 || index >= this.batchReports.length) return;
    this.currentBatchIndex = index;
    this.currentReport = this.batchReports[index];
    if (this.activeTab === "raw" && this.currentReport) {
      await this.loadRawContent(this.currentRawFormat);
    }
    this.renderShell();
  }

  private setupListeners() {
    document.addEventListener("mousedown", (e) => {
      const target = e.target as HTMLElement;
      if (e.button === 0 && target.closest(".titlebar") && !target.closest("button, .btn, .titlebar-actions, select")) {
        invoke("start_window_drag").catch(() => { });
      }
    });

    document.addEventListener("change", async (e) => {
      const target = e.target as HTMLElement;
      if (target.id === "quick-file-select") {
        const select = target as HTMLSelectElement;
        const idx = parseInt(select.value, 10);
        await this.switchBatchFile(idx);
      }
      if (target.id === "toggle-remember-window") {
        const checkbox = target as HTMLInputElement;
        try {
          this.settings = await invoke<AppSettings>("set_remember_window_state", { enabled: checkbox.checked });
          console.log("[app] updated settings:", this.settings);
        } catch (err) {
          console.error("Failed to save remember_window_state setting:", err);
        }
      }
    });

    document.addEventListener("click", async (e) => {
      const target = e.target as HTMLElement;

      // Settings modal toggles
      if (target.id === "btn-open-settings" || target.closest("#btn-open-settings")) {
        this.isSettingsOpen = true;
        this.renderShell();
        return;
      }
      if (target.id === "btn-close-settings" || target.id === "settings-modal-backdrop") {
        this.isSettingsOpen = false;
        this.renderShell();
        return;
      }
      if (target.id === "btn-reset-window") {
        try {
          this.settings = await invoke<AppSettings>("reset_window_geometry");
          target.innerText = "✓ Reset";
          setTimeout(() => { target.innerText = "Reset"; }, 1200);
        } catch (err) {
          console.error("Failed to reset window geometry:", err);
        }
        return;
      }

      // --- Multi-File Compare Actions ---
      if (target.id === "compare-add-slot-btn" || target.closest("#compare-add-slot-btn")) {
        await this.addCompareSlotDialog();
        return;
      }

      const quickAddBtn = target.closest(".btn-quick-add-compare") as HTMLElement | null;
      if (quickAddBtn) {
        const idx = parseInt(quickAddBtn.getAttribute("data-batch-index") || "0", 10);
        const report = this.batchReports[idx];
        if (report && this.compareSlots.length < 6) {
          const path = report.general?.file_path || `File #${idx + 1}`;
          const name = report.general?.file_name || path.split("/").pop() || `File #${idx + 1}`;
          if (!this.compareSlots.some((s) => s.path === path)) {
            this.compareSlots.push({ path, name, report });
            this.renderShell();
          }
        }
        return;
      }

      const changeSlotBtn = target.closest(".btn-change-slot") as HTMLElement | null;
      if (changeSlotBtn) {
        const slotIdx = parseInt(changeSlotBtn.getAttribute("data-slot-index") || "0", 10);
        await this.changeCompareSlotDialog(slotIdx);
        return;
      }

      const removeSlotBtn = target.closest(".btn-remove-slot") as HTMLElement | null;
      if (removeSlotBtn) {
        const slotIdx = parseInt(removeSlotBtn.getAttribute("data-slot-index") || "0", 10);
        if (slotIdx >= 0 && slotIdx < this.compareSlots.length) {
          this.compareSlots.splice(slotIdx, 1);
          this.renderShell();
        }
        return;
      }

      const filterPill = target.closest(".compare-filter-pill") as HTMLElement | null;
      if (filterPill) {
        const filter = filterPill.getAttribute("data-filter") as CompareFilter;
        if (filter && filter !== this.compareFilter) {
          this.compareFilter = filter;
          this.renderShell();
        }
        return;
      }

      if (target.id === "btn-clear-compare-search" || target.closest("#btn-clear-compare-search")) {
        this.compareSearchQuery = "";
        this.renderShell();
        return;
      }

      if (target.id === "btn-compare-clear" || target.closest("#btn-compare-clear")) {
        this.compareSlots = [];
        this.renderShell();
        return;
      }

      if (target.id === "btn-compare-export-csv" || target.closest("#btn-compare-export-csv")) {
        this.exportCompareCsv();
        return;
      }

      // Tree View header accordion toggle
      const treeHeader = target.closest(".tree-header") as HTMLElement | null;
      if (treeHeader) {
        const section = treeHeader.closest(".tree-section") as HTMLElement | null;
        if (section) {
          section.classList.toggle("collapsed");
        }
        return;
      }

      // Tree View search clear buttons
      if (target.id === "btn-tree-search-clear" || target.id === "btn-clear-tree-search") {
        this.treeSearchQuery = "";
        const treeInput = document.getElementById("tree-search-input") as HTMLInputElement | null;
        if (treeInput) treeInput.value = "";
        const container = document.getElementById("tree-sections-container");
        const badge = document.getElementById("tree-search-badge");
        if (this.currentReport && container) {
          const { html } = renderTreeSections(this.currentReport, "");
          container.innerHTML = html;
          if (badge) {
            badge.textContent = "Live Filter";
            badge.classList.remove("active");
          }
        }
        if (treeInput) treeInput.focus();
        return;
      }

      // Quick stepper buttons
      if (target.id === "btn-prev-file") {
        await this.switchBatchFile(this.currentBatchIndex - 1);
        return;
      }
      if (target.id === "btn-next-file") {
        await this.switchBatchFile(this.currentBatchIndex + 1);
        return;
      }

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
      if (target.id === "btn-open-folder" || target.id === "btn-empty-open-folder" || target.id === "batch-add-folder-btn") {
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

      // Batch row click
      const batchRow = target.closest(".batch-row") as HTMLElement | null;
      if (batchRow) {
        const idx = parseInt(batchRow.getAttribute("data-index") || "0", 10);
        if (this.batchReports[idx]) {
          this.currentBatchIndex = idx;
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
        this.currentBatchIndex = 0;
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
        const container = document.getElementById("tree-sections-container");
        const badge = document.getElementById("tree-search-badge");
        if (container && this.currentReport) {
          const { html, matchCount } = renderTreeSections(this.currentReport, this.treeSearchQuery);
          container.innerHTML = html;
          if (badge) {
            badge.textContent = this.treeSearchQuery.trim() ? `${matchCount} matches` : "Live Filter";
            if (this.treeSearchQuery.trim()) {
              badge.classList.add("active");
            } else {
              badge.classList.remove("active");
            }
          }
        }
      });
    }

    const compareInput = document.getElementById("compare-search-input") as HTMLInputElement | null;
    if (compareInput) {
      compareInput.addEventListener("input", () => {
        this.compareSearchQuery = compareInput.value;
        const mainView = document.getElementById("main-content-view");
        if (mainView && this.activeTab === "diff") {
          mainView.innerHTML = renderDiffView(this.compareSlots, this.compareFilter, this.compareSearchQuery, this.batchReports);
          this.attachViewEvents();
          const newInput = document.getElementById("compare-search-input") as HTMLInputElement | null;
          if (newInput) {
            newInput.focus();
            newInput.setSelectionRange(this.compareSearchQuery.length, this.compareSearchQuery.length);
          }
        }
      });
    }
  }

  private setupShortcuts() {
    window.addEventListener("keydown", async (e) => {
      const cmd = e.metaKey || e.ctrlKey;
      if (cmd && e.key === "o") { e.preventDefault(); await this.openFileDialog(); return; }
      if (cmd && e.key === "O") { e.preventDefault(); await this.openFolderDialog(); return; }
      if (cmd && e.key === ",") {
        e.preventDefault();
        this.isSettingsOpen = !this.isSettingsOpen;
        this.renderShell();
        return;
      }
      if (e.key === "Escape" && this.isSettingsOpen) {
        e.preventDefault();
        this.isSettingsOpen = false;
        this.renderShell();
        return;
      }

      // Quick stepper shortcuts: [ or Alt+Left for prev, ] or Alt+Right for next
      if (this.batchReports.length > 1) {
        if (e.key === "[" || (e.altKey && e.key === "ArrowLeft")) {
          e.preventDefault();
          await this.switchBatchFile(this.currentBatchIndex - 1);
          return;
        }
        if (e.key === "]" || (e.altKey && e.key === "ArrowRight")) {
          e.preventDefault();
          await this.switchBatchFile(this.currentBatchIndex + 1);
          return;
        }
      }

      if (cmd && e.key >= "1" && e.key <= "5") {
        e.preventDefault();
        const tabs: ("summary" | "tree" | "raw" | "diff" | "batch")[] = [
          "summary",
          "tree",
          "raw",
          "diff",
          "batch",
        ];
        this.activeTab = tabs[parseInt(e.key, 10) - 1];
        if (this.activeTab === "raw" && this.currentReport) {
          await this.loadRawContent(this.currentRawFormat);
        }
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
            // Check if it's a folder scan or a single file
            try {
              const folderReports = await invoke<MediaReport[]>("scan_folder", { folderPath: paths[0] });
              if (folderReports && folderReports.length > 0) {
                this.batchReports = folderReports;
                this.currentBatchIndex = 0;
                this.currentReport = folderReports[0];
                this.renderShell();
                return;
              }
            } catch {
              // Not a directory or scan failed, fall through to loadFile
            }
            await this.loadFile(paths[0]);
          } else {
            await this.processBatch(paths);
            this.currentBatchIndex = 0;
            this.currentReport = this.batchReports[0];
            this.renderShell();
          }
        }
      });
      console.log("[app] drag-drop listener registered");
    } catch (err) {
      console.warn("[app] drag-drop setup failed (web preview mode?):", err);
    }
  }

  // --- Multi-file compare dialogs ---
  private async addCompareSlotDialog() {
    if (this.compareSlots.length >= 6) return;
    const remaining = 6 - this.compareSlots.length;
    try {
      const exts = this.supportedExtensions.length > 0
        ? this.supportedExtensions
        : ["mp4", "mkv", "mov", "avi", "wav", "flac", "mp3", "aac", "m4a", "webm", "ts", "ogg", "opus", "dts", "ac3", "mpc"];
      const selected = await open({
        multiple: remaining > 1,
        filters: [{
          name: "Media Files",
          extensions: exts,
        }],
      });
      if (!selected) return;
      const paths = (Array.isArray(selected) ? selected : [selected]).slice(0, remaining);
      for (const path of paths) {
        if (typeof path === "string" && !this.compareSlots.some((s) => s.path === path)) {
          const report = await invoke<MediaReport>("inspect_file", { path });
          const name = report.general?.file_name || path.split("/").pop() || "Media File";
          this.compareSlots.push({ path, name, report });
        }
      }
      this.renderShell();
    } catch (err) {
      console.error("[app] addCompareSlot error:", err);
      this.showError("Failed to add file for comparison: " + err);
    }
  }

  private async changeCompareSlotDialog(slotIdx: number) {
    if (slotIdx < 0 || slotIdx >= this.compareSlots.length) return;
    const path = await this.pickSingleFile();
    if (path) {
      try {
        const report = await invoke<MediaReport>("inspect_file", { path });
        const name = report.general?.file_name || path.split("/").pop() || "Media File";
        this.compareSlots[slotIdx] = { path, name, report };
        this.renderShell();
      } catch (err) {
        console.error("[app] changeCompareSlot error:", err);
        this.showError("Failed to inspect file for comparison: " + err);
      }
    }
  }

  private exportCompareCsv() {
    if (this.compareSlots.length < 2) return;
    const rows = buildComparisonRows(this.compareSlots);
    const header = [
      "Category",
      "Parameter",
      ...this.compareSlots.map((s, i) => `File ${i + 1} (${s.name})`),
      "Is_Difference",
    ];

    const csvLines = [
      header.map((h) => `"${h.replace(/"/g, '""')}"`).join(","),
      ...rows.map((r) => [
        `"${r.category.replace(/"/g, '""')}"`,
        `"${r.field.replace(/"/g, '""')}"`,
        ...r.values.map((v) => `"${v.replace(/"/g, '""')}"`),
        r.isDiff ? "Yes" : "No",
      ].join(",")),
    ];

    const blob = new Blob([csvLines.join("\n")], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `mediainfo_comparison_${Date.now()}.csv`;
    link.click();
  }

  // --- File dialogs using Tauri dialog plugin ---
  private async openFileDialog() {
    console.log("[app] openFileDialog()");
    try {
      const exts = this.supportedExtensions.length > 0
        ? this.supportedExtensions
        : ["mp4", "mkv", "mov", "avi", "wav", "flac", "mp3", "aac", "m4a", "webm", "ts", "ogg", "opus", "dts", "ac3", "mpc"];
      const selected = await open({
        multiple: true,
        filters: [{
          name: "Media Files",
          extensions: exts,
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
        this.currentBatchIndex = 0;
        this.currentReport = this.batchReports[0];
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
      await this.loadFolder(selected);
    } catch (err) {
      console.error("[app] openFolderDialog error:", err);
      this.showError("Open folder dialog error: " + err);
    }
  }

  private async loadFolder(folderPath: string) {
    console.log("[app] loadFolder()", folderPath);
    try {
      const reports = await invoke<MediaReport[]>("scan_folder", { folderPath });
      console.log("[app] scan_folder reports:", reports.length);
      if (reports.length === 0) {
        this.showError(`No supported media files found in: ${folderPath}`);
        return;
      }
      this.batchReports = reports;
      this.currentBatchIndex = 0;
      this.currentReport = reports[0];
      this.renderShell();
    } catch (err) {
      console.error("[app] loadFolder error:", err);
      this.showError("Folder scan error: " + err);
    }
  }

  private async pickSingleFile(): Promise<string | null> {
    try {
      const exts = this.supportedExtensions.length > 0
        ? this.supportedExtensions
        : ["mp4", "mkv", "mov", "avi", "wav", "flac", "mp3", "aac", "m4a", "webm", "ts", "ogg", "opus", "dts", "ac3", "mpc"];
      const selected = await open({
        multiple: false,
        filters: [{
          name: "Media Files",
          extensions: exts,
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
