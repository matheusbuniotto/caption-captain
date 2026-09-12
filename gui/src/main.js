// No bundler serves this file (Tauri loads it as a plain static asset), so
// bare npm-package imports like "@tauri-apps/api/core" can't resolve in the
// webview. Use the `window.__TAURI__` globals instead (enabled via
// `withGlobalTauri` in tauri.conf.json).
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWebview } = window.__TAURI__.webview;
const { open: openFileDialog } = window.__TAURI__.dialog;

// DOM Elements
const banner = document.getElementById("banner");
const dropZone = document.getElementById("drop-zone");
const browseButton = document.getElementById("browse-button");
const formatsChips = document.getElementById("formats-chips");

// Selection section
const selectionSection = document.getElementById("selection-section");
const selectionCountBadge = document.getElementById("selection-count-badge");
const addMoreButton = document.getElementById("add-more-button");
const clearSelectionButton = document.getElementById("clear-selection-button");
const selectedFilesList = document.getElementById("selected-files-list");
const langInput = document.getElementById("lang-input");
const embedCheckbox = document.getElementById("embed-checkbox");
const startButton = document.getElementById("start-button");
const cancelSelectionButton = document.getElementById("cancel-selection-button");

// Progress section
const progressSection = document.getElementById("progress-section");
const progressBatchTitle = document.getElementById("progress-batch-title");
const batchProgressPct = document.getElementById("batch-progress-pct");
const progressBarFill = document.getElementById("progress-bar-fill");
const progressCurrentFilename = document.getElementById("progress-current-filename");
const stageItems = document.querySelectorAll("#progress-stages li");
const progressQueueList = document.getElementById("progress-queue-list");

// Result section
const resultSection = document.getElementById("result-section");
const resultSummaryText = document.getElementById("result-summary-text");
const resultItemsList = document.getElementById("result-items-list");
const processAnotherButton = document.getElementById("process-another-button");

// Error section
const errorSection = document.getElementById("error-section");
const errorMessage = document.getElementById("error-message");
const errorDismissButton = document.getElementById("error-dismiss-button");

// App State
let acceptedExtensions = ["mp4", "mov", "mkv", "avi", "m4v"];
let selectedVideos = [];
let isProcessing = false;
let lastBatchResults = [];

function showBanner(text) {
  banner.textContent = text;
  banner.classList.remove("hidden");
}

function hideBanner() {
  banner.classList.add("hidden");
}

function setSection(section) {
  for (const el of [dropZone, selectionSection, progressSection, resultSection, errorSection]) {
    el.classList.toggle("hidden", el !== section);
  }
}

function filenameOf(path) {
  return path.split("/").pop();
}

function extensionOf(filename) {
  const dot = filename.lastIndexOf(".");
  return dot === -1 ? "" : filename.slice(dot + 1).toLowerCase();
}

function isAcceptedVideo(path) {
  return acceptedExtensions.includes(extensionOf(path));
}

function resetProgressUi() {
  progressBarFill.style.width = "0%";
  batchProgressPct.textContent = "0%";
  for (const item of stageItems) {
    item.classList.remove("active", "done");
  }
  progressQueueList.innerHTML = "";
}

function markStage(stageName) {
  const order = ["ExtractingAudio", "Transcribing", "Embedding"];
  const idx = order.indexOf(stageName);
  if (idx === -1) return;
  order.forEach((name, i) => {
    const item = document.querySelector(`[data-stage="${name}"]`);
    if (item) {
      item.classList.toggle("done", i < idx);
      item.classList.toggle("active", i === idx);
    }
  });
}

function renderSelectionList() {
  selectedFilesList.innerHTML = "";
  const count = selectedVideos.length;
  selectionCountBadge.textContent = `${count} file${count === 1 ? "" : "s"}`;
  startButton.textContent = `Start Captioning (${count})`;

  selectedVideos.forEach((path, idx) => {
    const name = filenameOf(path);
    const ext = extensionOf(name);

    const item = document.createElement("div");
    item.className = "file-item";
    item.innerHTML = `
      <div class="file-item-left">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="23 7 16 12 23 17 23 7"></polygon>
          <rect x="1" y="5" width="15" height="14" rx="2" ry="2"></rect>
        </svg>
        <span class="file-name">${name}</span>
        <span class="chip">.${ext}</span>
      </div>
      <button type="button" class="btn-remove-item" data-index="${idx}" title="Remove file">✕</button>
    `;

    item.querySelector(".btn-remove-item").addEventListener("click", (e) => {
      e.stopPropagation();
      removeVideo(idx);
    });

    selectedFilesList.appendChild(item);
  });
}

function addVideos(paths) {
  if (isProcessing) {
    showBanner("Batch processing is already running.");
    return;
  }

  const validPaths = paths.filter(isAcceptedVideo);
  const invalidCount = paths.length - validPaths.length;

  if (invalidCount > 0) {
    showBanner(`Skipped ${invalidCount} unsupported file(s). Accepted: ${acceptedExtensions.map((e) => "." + e).join(" ")}`);
  } else {
    hideBanner();
  }

  if (validPaths.length === 0 && selectedVideos.length === 0) {
    return;
  }

  // Add without duplicates
  for (const path of validPaths) {
    if (!selectedVideos.includes(path)) {
      selectedVideos.push(path);
    }
  }

  if (selectedVideos.length > 0) {
    renderSelectionList();
    setSection(selectionSection);
  }
}

function removeVideo(index) {
  selectedVideos.splice(index, 1);
  if (selectedVideos.length === 0) {
    hideBanner();
    setSection(dropZone);
  } else {
    renderSelectionList();
  }
}

function renderQueueList() {
  progressQueueList.innerHTML = "";
  selectedVideos.forEach((path, idx) => {
    const item = document.createElement("div");
    item.className = "queue-item";
    item.id = `queue-item-${idx}`;
    item.innerHTML = `
      <span>${idx + 1}. ${filenameOf(path)}</span>
      <span class="queue-status-badge chip" id="queue-status-${idx}">Queued</span>
    `;
    progressQueueList.appendChild(item);
  });
}

function updateQueueItem(index, status, badgeClass = "chip") {
  const badge = document.getElementById(`queue-status-${index}`);
  const row = document.getElementById(`queue-item-${index}`);
  if (badge) {
    badge.textContent = status;
    badge.className = `queue-status-badge ${badgeClass}`;
  }
  if (row) {
    row.classList.toggle("current", status === "Active");
  }
}

async function startBatchProcessing() {
  if (selectedVideos.length === 0 || isProcessing) return;

  hideBanner();
  isProcessing = true;
  resetProgressUi();
  renderQueueList();

  const total = selectedVideos.length;
  progressBatchTitle.textContent = `Processing video 1 of ${total}`;
  progressCurrentFilename.textContent = filenameOf(selectedVideos[0]);
  updateQueueItem(0, "Active", "badge-pill badge-pill-soft");

  setSection(progressSection);

  const lang = langInput.value.trim() || null;
  const embed = embedCheckbox.checked;

  try {
    const results = await invoke("process_batch", {
      videoPaths: selectedVideos,
      lang,
      embed,
    });
    lastBatchResults = results;
    showResults(results);
  } catch (err) {
    showError(String(err));
  } finally {
    isProcessing = false;
  }
}

function showResults(results) {
  const total = results.length;
  const succeeded = results.filter((r) => r.success).length;
  const withWarnings = results.filter((r) => r.success && r.warning).length;
  const failed = results.filter((r) => !r.success).length;

  if (failed === 0 && withWarnings === 0) {
    resultSummaryText.textContent = `All ${total} video${total === 1 ? "" : "s"} captioned and muxed successfully.`;
  } else {
    const parts = [];
    parts.push(`${succeeded} succeeded`);
    if (withWarnings > 0) parts.push(`${withWarnings} sidecar-only`);
    if (failed > 0) parts.push(`${failed} failed`);
    resultSummaryText.textContent = `Batch finished: ${parts.join(", ")} out of ${total} video${total === 1 ? "" : "s"}.`;
  }

  resultItemsList.innerHTML = "";
  results.forEach((item) => {
    const name = filenameOf(item.video_path);
    const card = document.createElement("div");
    card.className = "result-item-card";

    let statusBadge = "";
    if (!item.success) {
      statusBadge = '<span class="badge-pill badge-pill-danger">Failed</span>';
    } else if (item.warning) {
      statusBadge = '<span class="badge-pill badge-pill-warning">Sidecar Only (.srt)</span>';
    } else {
      statusBadge = '<span class="badge-pill badge-pill-soft">Embedded</span>';
    }

    let extraNote = "";
    if (item.error) {
      extraNote = `<div class="result-error-text">${item.error}</div>`;
    } else if (item.warning) {
      extraNote = `<div class="result-warning-text">${item.warning}</div>`;
    }

    let actionsHtml = "";
    if (item.success) {
      actionsHtml = `
        <div class="result-item-actions">
          <button type="button" class="btn-sm-pill btn-reveal" title="Reveal in Finder">Reveal</button>
          ${item.muxed_path ? '<button type="button" class="btn-sm-pill btn-play" title="Open in Player">Play</button>' : ""}
        </div>
      `;
    }

    card.innerHTML = `
      <div class="result-item-top">
        <span class="result-filename">${name}</span>
        ${statusBadge}
      </div>
      ${extraNote}
      <div class="result-item-bottom">
        <div class="result-meta">
          ${item.language ? `<span class="chip">Language: ${item.language}</span>` : ""}
        </div>
        ${actionsHtml}
      </div>
    `;

    const revealBtn = card.querySelector(".btn-reveal");
    if (revealBtn) {
      revealBtn.addEventListener("click", () => {
        invoke("reveal_in_finder", { path: item.muxed_path ?? item.srt_path });
      });
    }

    const playBtn = card.querySelector(".btn-play");
    if (playBtn && item.muxed_path) {
      playBtn.addEventListener("click", () => {
        invoke("open_in_player", { path: item.muxed_path });
      });
    }

    resultItemsList.appendChild(card);
  });

  setSection(resultSection);
}

function showError(message) {
  errorMessage.textContent = message;
  setSection(errorSection);
}

// Listen to stage events from Rust pipeline
listen("pipeline-stage", (event) => {
  const stage = event.payload.stage;
  markStage(stage);
});

// Listen to batch item start
listen("batch-item-start", (event) => {
  const { index, total, video_path } = event.payload;
  progressBatchTitle.textContent = `Processing video ${index + 1} of ${total}`;
  progressCurrentFilename.textContent = filenameOf(video_path);

  const pct = Math.round((index / total) * 100);
  progressBarFill.style.width = `${pct}%`;
  batchProgressPct.textContent = `${pct}%`;

  // Reset stage dots for this item
  for (const item of stageItems) {
    item.classList.remove("active", "done");
  }

  updateQueueItem(index, "Active", "badge-pill badge-pill-soft");
});

// Listen to batch item complete
listen("batch-item-complete", (event) => {
  const result = event.payload;
  const idx = selectedVideos.indexOf(result.video_path);
  if (idx !== -1) {
    if (!result.success) {
      updateQueueItem(idx, "Failed", "badge-pill badge-pill-danger");
    } else if (result.warning) {
      updateQueueItem(idx, "Sidecar only", "badge-pill badge-pill-warning");
    } else {
      updateQueueItem(idx, "Done", "badge-pill badge-pill-soft");
    }
  }
});

// Drag-and-drop onto the window (Tauri's native file-drop event)
getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type === "over") {
    dropZone.classList.add("drag-over");
  } else if (event.payload.type === "drop") {
    dropZone.classList.remove("drag-over");
    const paths = event.payload.paths || [];
    if (paths.length > 0) {
      addVideos(paths);
    }
  } else {
    dropZone.classList.remove("drag-over");
  }
});

async function openFilePicker() {
  const filters = [{ name: "Video", extensions: acceptedExtensions }];
  const selected = await openFileDialog({ multiple: true, filters });
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected];
    addVideos(paths);
  }
}

browseButton.addEventListener("click", openFilePicker);
addMoreButton.addEventListener("click", openFilePicker);

clearSelectionButton.addEventListener("click", () => {
  selectedVideos = [];
  hideBanner();
  setSection(dropZone);
});

cancelSelectionButton.addEventListener("click", () => {
  selectedVideos = [];
  hideBanner();
  setSection(dropZone);
});

startButton.addEventListener("click", startBatchProcessing);

processAnotherButton.addEventListener("click", () => {
  selectedVideos = [];
  lastBatchResults = [];
  hideBanner();
  setSection(dropZone);
});

errorDismissButton.addEventListener("click", () => {
  selectedVideos = [];
  hideBanner();
  setSection(dropZone);
});

async function init() {
  try {
    acceptedExtensions = await invoke("accepted_extensions");
  } catch {
    // Keep the hardcoded default if command isn't reachable yet.
  }
  if (formatsChips) {
    formatsChips.innerHTML = acceptedExtensions
      .map((ext) => `<span class="chip">.${ext}</span>`)
      .join(" ");
  }
}

init();
