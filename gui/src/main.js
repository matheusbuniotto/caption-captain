// No bundler serves this file (Tauri loads it as a plain static asset), so
// bare npm-package imports like "@tauri-apps/api/core" can't resolve in the
// webview. Use the `window.__TAURI__` globals instead (enabled via
// `withGlobalTauri` in tauri.conf.json).
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWebview } = window.__TAURI__.webview;
const { open: openFileDialog } = window.__TAURI__.dialog;

const STAGE_LABELS = {
  ExtractingAudio: "Extracting Audio",
  Transcribing: "Transcribing",
  Embedding: "Embedding",
};

// Elements
const banner = document.getElementById("banner");
const dropZone = document.getElementById("drop-zone");
const formatsList = document.getElementById("formats-list");
const browseButton = document.getElementById("browse-button");
const selectionSection = document.getElementById("selection-section");
const selectedFilename = document.getElementById("selected-filename");
const langInput = document.getElementById("lang-input");
const startButton = document.getElementById("start-button");
const cancelSelectionButton = document.getElementById("cancel-selection-button");
const progressSection = document.getElementById("progress-section");
const progressVideoName = document.getElementById("progress-video-name");
const stageItems = document.querySelectorAll("#progress-stages li");
const resultSection = document.getElementById("result-section");
const resultMessage = document.getElementById("result-message");
const resultLanguage = document.getElementById("result-language");
const revealButton = document.getElementById("reveal-button");
const openButton = document.getElementById("open-button");
const processAnotherButton = document.getElementById("process-another-button");
const errorSection = document.getElementById("error-section");
const errorMessage = document.getElementById("error-message");
const errorDismissButton = document.getElementById("error-dismiss-button");

let acceptedExtensions = ["mp4", "mov", "mkv", "avi", "m4v"];
let isProcessing = false;
let pendingVideoPath = null;
let lastResult = null;

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

function extensionOf(filename) {
  const dot = filename.lastIndexOf(".");
  return dot === -1 ? "" : filename.slice(dot + 1).toLowerCase();
}

function isAcceptedVideo(path) {
  return acceptedExtensions.includes(extensionOf(path));
}

function resetProgressUi() {
  for (const item of stageItems) {
    item.classList.remove("active", "done");
  }
}

function markStage(stageName) {
  const order = ["ExtractingAudio", "Transcribing", "Embedding"];
  const idx = order.indexOf(stageName);
  if (idx === -1) return;
  order.forEach((name, i) => {
    const item = document.querySelector(`[data-stage="${name}"]`);
    item.classList.toggle("done", i < idx);
    item.classList.toggle("active", i === idx);
  });
}

// Drop/browse only *selects* a video; processing waits for the user to
// confirm (and optionally set a language override) on the selection
// screen, so a drop doesn't race ahead of the user setting options.
function selectVideo(videoPath) {
  if (isProcessing || pendingVideoPath) {
    showBanner("Already handling a video — finish or cancel it first.");
    return;
  }
  if (!isAcceptedVideo(videoPath)) {
    showBanner(`Unsupported file type. Accepted: ${acceptedExtensions.map((e) => "." + e).join(" ")}`);
    return;
  }

  hideBanner();
  pendingVideoPath = videoPath;
  selectedFilename.textContent = videoPath.split("/").pop();
  langInput.value = "";
  setSection(selectionSection);
}

async function startProcessing(videoPath) {
  hideBanner();
  isProcessing = true;
  resetProgressUi();
  progressVideoName.textContent = videoPath.split("/").pop();
  setSection(progressSection);

  try {
    const result = await invoke("process_video", {
      videoPath,
      lang: langInput.value.trim() || null,
      embed: true,
    });
    lastResult = result;
    showResult(result);
  } catch (err) {
    showError(String(err));
  } finally {
    isProcessing = false;
  }
}

function showResult(result) {
  if (result.warning) {
    resultMessage.textContent = `Done. ${result.warning}`;
    openButton.classList.add("hidden");
  } else {
    resultMessage.textContent = "Captioning complete.";
    openButton.classList.remove("hidden");
  }
  resultLanguage.textContent = `Language: ${result.language}`;
  setSection(resultSection);
}

function showError(message) {
  errorMessage.textContent = message;
  setSection(errorSection);
}

// Progress events streamed from the Rust pipeline.
listen("pipeline-stage", (event) => {
  const stage = event.payload.stage;
  markStage(stage);
});

// Drag-and-drop onto the window (Tauri's native file-drop event).
getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type === "over") {
    dropZone.classList.add("drag-over");
  } else if (event.payload.type === "drop") {
    dropZone.classList.remove("drag-over");
    const [path] = event.payload.paths;
    if (path) selectVideo(path);
  } else {
    dropZone.classList.remove("drag-over");
  }
});

browseButton.addEventListener("click", async () => {
  const filters = [{ name: "Video", extensions: acceptedExtensions }];
  const selected = await openFileDialog({ multiple: false, filters });
  if (selected) selectVideo(selected);
});

startButton.addEventListener("click", () => {
  const videoPath = pendingVideoPath;
  pendingVideoPath = null;
  startProcessing(videoPath);
});

cancelSelectionButton.addEventListener("click", () => {
  pendingVideoPath = null;
  setSection(dropZone);
});

revealButton.addEventListener("click", () => {
  if (lastResult) invoke("reveal_in_finder", { path: lastResult.muxed_path ?? lastResult.srt_path });
});

openButton.addEventListener("click", () => {
  if (lastResult?.muxed_path) invoke("open_in_player", { path: lastResult.muxed_path });
});

processAnotherButton.addEventListener("click", () => {
  lastResult = null;
  setSection(dropZone);
});

errorDismissButton.addEventListener("click", () => {
  setSection(dropZone);
});

async function init() {
  try {
    acceptedExtensions = await invoke("accepted_extensions");
  } catch {
    // Keep the hardcoded default above if the command isn't reachable yet.
  }
  formatsList.textContent = `Accepted: ${acceptedExtensions.map((e) => "." + e).join(" ")}`;
}

init();
