import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";

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
const langInput = document.getElementById("lang-input");
const progressSection = document.getElementById("progress-section");
const progressVideoName = document.getElementById("progress-video-name");
const stageItems = document.querySelectorAll("#progress-stages li");
const resultSection = document.getElementById("result-section");
const resultMessage = document.getElementById("result-message");
const revealButton = document.getElementById("reveal-button");
const openButton = document.getElementById("open-button");
const processAnotherButton = document.getElementById("process-another-button");
const errorSection = document.getElementById("error-section");
const errorMessage = document.getElementById("error-message");
const errorDismissButton = document.getElementById("error-dismiss-button");

let acceptedExtensions = ["mp4", "mov", "mkv", "avi", "m4v"];
let isProcessing = false;
let lastResult = null;

function showBanner(text) {
  banner.textContent = text;
  banner.classList.remove("hidden");
}

function hideBanner() {
  banner.classList.add("hidden");
}

function setSection(section) {
  for (const el of [dropZone, progressSection, resultSection, errorSection]) {
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

async function startProcessing(videoPath) {
  if (isProcessing) {
    showBanner("Still processing the previous video — please wait.");
    return;
  }
  if (!isAcceptedVideo(videoPath)) {
    showBanner(`Unsupported file type. Accepted: ${acceptedExtensions.map((e) => "." + e).join(" ")}`);
    return;
  }

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
    if (path) startProcessing(path);
  } else {
    dropZone.classList.remove("drag-over");
  }
});

browseButton.addEventListener("click", async () => {
  const filters = [{ name: "Video", extensions: acceptedExtensions }];
  const selected = await openFileDialog({ multiple: false, filters });
  if (selected) startProcessing(selected);
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
