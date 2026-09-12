use capcap::pipeline::{self, PipelineOptions, PipelineStage};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Runtime, State};

/// File extensions the GUI accepts, shown to the user before they drop a
/// file (user story #3) and used by the frontend to reject anything else.
pub const ACCEPTED_EXTENSIONS: [&str; 5] = ["mp4", "mov", "mkv", "avi", "m4v"];

/// Tracks whether a video is currently being processed, so a drop or a
/// second `process_video` call while one is in flight is refused instead of
/// racing two pipeline runs (user story #11).
pub struct ProcessingState(AtomicBool);

impl Default for ProcessingState {
    fn default() -> Self {
        Self(AtomicBool::new(false))
    }
}

/// JSON-friendly mirror of `capcap::pipeline::PipelineStage`, emitted to the
/// frontend as `"pipeline-stage"` events. Kept separate from the library
/// type since that type has no serde impls and gains one only for this
/// caller would be an odd coupling for the CLI-facing lib crate.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage")]
pub enum StageEvent {
    ExtractingAudio,
    Transcribing,
    WritingSidecar {
        srt_path: String,
    },
    Embedding,
    SkippedEmbed {
        reason: String,
    },
    Done {
        srt_path: String,
        muxed_path: Option<String>,
        language: String,
    },
}

impl From<&PipelineStage> for StageEvent {
    fn from(stage: &PipelineStage) -> Self {
        match stage {
            PipelineStage::ExtractingAudio => StageEvent::ExtractingAudio,
            PipelineStage::Transcribing => StageEvent::Transcribing,
            PipelineStage::WritingSidecar { srt_path } => StageEvent::WritingSidecar {
                srt_path: srt_path.display().to_string(),
            },
            PipelineStage::Embedding => StageEvent::Embedding,
            PipelineStage::SkippedEmbed { reason } => StageEvent::SkippedEmbed {
                reason: reason.clone(),
            },
            PipelineStage::Done {
                srt_path,
                muxed_path,
                language,
            } => StageEvent::Done {
                srt_path: srt_path.display().to_string(),
                muxed_path: muxed_path.as_ref().map(|p| p.display().to_string()),
                language: language.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessVideoResult {
    pub srt_path: String,
    pub muxed_path: Option<String>,
    pub warning: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BatchItemResult {
    pub video_path: String,
    pub srt_path: Option<String>,
    pub muxed_path: Option<String>,
    pub warning: Option<String>,
    pub language: Option<String>,
    pub error: Option<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchProgressPayload {
    pub index: usize,
    pub total: usize,
    pub video_path: String,
    pub stage: StageEvent,
}

/// The formats the drop zone / file picker accept, for the frontend to
/// render before the user drops anything.
#[tauri::command]
fn accepted_extensions() -> Vec<&'static str> {
    ACCEPTED_EXTENSIONS.to_vec()
}

/// Runs the shared pipeline against `video_path`, forwarding each
/// `PipelineStage` to the frontend as a `"pipeline-stage"` event, and
/// returning the final srt/muxed paths (plus a sidecar-only warning, if
/// any) for the success screen. Refuses to start a second run while one is
/// already in flight.
// Generic over `Runtime` (rather than hardcoding the real `tauri::Wry`
// runtime) so tests can call this exact function with `tauri::test`'s
// `MockRuntime` instead of needing a real window.
#[tauri::command]
async fn process_video<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ProcessingState>,
    video_path: String,
    lang: Option<String>,
    embed: bool,
) -> Result<ProcessVideoResult, String> {
    if state.0.swap(true, Ordering::SeqCst) {
        return Err("a video is already being processed".to_string());
    }

    let result = run_pipeline(app, video_path, lang, embed).await;
    state.0.store(false, Ordering::SeqCst);
    result
}

/// Runs the pipeline sequentially over a batch of `video_paths`, emitting
/// progress events for each stage and each batch item.
#[tauri::command]
async fn process_batch<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ProcessingState>,
    video_paths: Vec<String>,
    lang: Option<String>,
    embed: bool,
) -> Result<Vec<BatchItemResult>, String> {
    if video_paths.is_empty() {
        return Err("no videos provided".to_string());
    }

    if state.0.swap(true, Ordering::SeqCst) {
        return Err("a video is already being processed".to_string());
    }

    struct Guard<'a>(&'a AtomicBool);
    impl<'a> Drop for Guard<'a> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = Guard(&state.0);

    let total = video_paths.len();
    let mut results = Vec::with_capacity(total);

    for (index, video_path) in video_paths.into_iter().enumerate() {
        let app_handle = app.clone();
        let current_path = video_path.clone();
        let current_lang = lang.clone();

        let _ = app.emit(
            "batch-item-start",
            serde_json::json!({
                "index": index,
                "total": total,
                "video_path": &video_path,
            }),
        );

        let run_result = tauri::async_runtime::spawn_blocking(move || {
            let options = PipelineOptions {
                lang: current_lang,
                no_embed: !embed,
            };
            let app_for_stages = app_handle.clone();
            let path_for_stages = current_path.clone();
            pipeline::process_video(
                std::path::Path::new(&current_path),
                &options,
                move |stage| {
                    let stage_event = StageEvent::from(&stage);
                    let _ = app_for_stages.emit("pipeline-stage", &stage_event);
                    let _ = app_for_stages.emit(
                        "batch-progress",
                        BatchProgressPayload {
                            index,
                            total,
                            video_path: path_for_stages.clone(),
                            stage: stage_event,
                        },
                    );
                },
            )
        })
        .await
        .map_err(|e| format!("pipeline task panicked: {e}"));

        match run_result {
            Ok(Ok(output)) => {
                let item = BatchItemResult {
                    video_path: video_path.clone(),
                    srt_path: Some(output.srt_path.display().to_string()),
                    muxed_path: output.muxed_path.map(|p| p.display().to_string()),
                    warning: output.warning,
                    language: Some(output.language),
                    error: None,
                    success: true,
                };
                let _ = app.emit("batch-item-complete", &item);
                results.push(item);
            }
            Ok(Err(e)) => {
                let item = BatchItemResult {
                    video_path: video_path.clone(),
                    srt_path: None,
                    muxed_path: None,
                    warning: None,
                    language: None,
                    error: Some(format!("{e:#}")),
                    success: false,
                };
                let _ = app.emit("batch-item-complete", &item);
                results.push(item);
            }
            Err(e) => {
                let item = BatchItemResult {
                    video_path: video_path.clone(),
                    srt_path: None,
                    muxed_path: None,
                    warning: None,
                    language: None,
                    error: Some(e),
                    success: false,
                };
                let _ = app.emit("batch-item-complete", &item);
                results.push(item);
            }
        }
    }

    Ok(results)
}

/// The actual pipeline call, split out from `process_video` so it can be
/// exercised in tests without an `AppHandle`'s state-guard bookkeeping.
async fn run_pipeline<R: Runtime>(
    app: AppHandle<R>,
    video_path: String,
    lang: Option<String>,
    embed: bool,
) -> Result<ProcessVideoResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let options = PipelineOptions {
            lang,
            no_embed: !embed,
        };
        pipeline::process_video(std::path::Path::new(&video_path), &options, |stage| {
            let _ = app.emit("pipeline-stage", StageEvent::from(&stage));
        })
    })
    .await
    .map_err(|e| format!("pipeline task panicked: {e}"))?
    .map(|output| ProcessVideoResult {
        srt_path: output.srt_path.display().to_string(),
        muxed_path: output.muxed_path.map(|p| p.display().to_string()),
        warning: output.warning,
        language: output.language,
    })
    .map_err(|e| format!("{e:#}"))
}

/// Reveals `path` in the platform file manager (Finder on macOS, Explorer on Windows, xdg-open on Linux).
#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("failed to reveal {path} in Finder: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("failed to reveal {path} in Explorer: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("failed to open file manager for {path}: {e}"))?;
    }
    Ok(())
}

/// Opens `path` in the platform-default player (QuickTime on macOS, default video player on Windows/Linux).
#[tauri::command]
fn open_in_player(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to open {path}: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .spawn()
            .map_err(|e| format!("failed to open {path}: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("failed to open {path}: {e}"))?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ProcessingState::default())
        .invoke_handler(tauri::generate_handler![
            accepted_extensions,
            process_video,
            process_batch,
            reveal_in_finder,
            open_in_player
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    /// Builds a headless Tauri app (no window) so `process_video`'s
    /// Rust-side logic — a plain async function taking an `AppHandle` and
    /// `State` — can be called directly, per this ticket's testing
    /// decision to avoid WebDriver/GUI automation entirely.
    fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .manage(ProcessingState::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("failed to build mock tauri app")
    }

    fn fixture_copy(workdir: &std::path::Path, fixture: &str) -> std::path::PathBuf {
        let dest = workdir.join(fixture);
        std::fs::copy(format!("../../tests/fixtures/{fixture}"), &dest).unwrap();
        dest
    }

    #[tokio::test]
    async fn process_video_calls_the_shared_pipeline_and_returns_its_output() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video = fixture_copy(workdir.path(), "spoken-word.mp4");

        let result = process_video(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            video.to_string_lossy().to_string(),
            None,
            true,
        )
        .await
        .expect("pipeline should succeed on a real fixture video");

        assert!(std::path::Path::new(&result.srt_path).is_file());
        assert!(result.muxed_path.is_some());
        assert!(result.warning.is_none());
    }

    #[tokio::test]
    async fn process_video_forwards_the_sidecar_only_warning() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video = fixture_copy(workdir.path(), "spoken-word.avi");

        let result = process_video(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            video.to_string_lossy().to_string(),
            None,
            true,
        )
        .await
        .unwrap();

        assert!(result.muxed_path.is_none());
        assert!(result.warning.unwrap().contains("avi"));
    }

    #[tokio::test]
    async fn process_video_refuses_a_second_call_while_one_is_in_flight() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video = fixture_copy(workdir.path(), "spoken-word.mp4");

        // Simulate "already processing" directly on the shared state,
        // exactly what a real in-flight `process_video` call would leave
        // set for the duration of the pipeline run.
        app.state::<ProcessingState>()
            .0
            .store(true, Ordering::SeqCst);

        let result = process_video(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            video.to_string_lossy().to_string(),
            None,
            true,
        )
        .await;

        assert!(result.is_err(), "expected a busy error, got {result:?}");
    }

    #[tokio::test]
    async fn process_batch_processes_multiple_videos() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video1 = fixture_copy(workdir.path(), "spoken-word.mp4");
        let video2 = fixture_copy(workdir.path(), "spoken-word.avi");

        let results = process_batch(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            vec![
                video1.to_string_lossy().to_string(),
                video2.to_string_lossy().to_string(),
            ],
            None,
            true,
        )
        .await
        .expect("batch processing should succeed");

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[0].muxed_path.is_some());
        assert!(results[0].srt_path.is_some());

        assert!(results[1].success);
        assert!(results[1].muxed_path.is_none());
        assert!(results[1].warning.as_ref().unwrap().contains("avi"));
    }

    #[tokio::test]
    async fn process_batch_handles_errors_gracefully() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video1 = fixture_copy(workdir.path(), "spoken-word.mp4");
        let nonexistent = workdir.path().join("nonexistent.mp4");

        let results = process_batch(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            vec![
                video1.to_string_lossy().to_string(),
                nonexistent.to_string_lossy().to_string(),
            ],
            None,
            true,
        )
        .await
        .expect("batch processing returns list with individual error status");

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[1].error.is_some());
    }

    #[tokio::test]
    async fn process_batch_refuses_when_busy() {
        let app = mock_app();
        let workdir = tempfile::tempdir().unwrap();
        let video = fixture_copy(workdir.path(), "spoken-word.mp4");

        app.state::<ProcessingState>()
            .0
            .store(true, Ordering::SeqCst);

        let result = process_batch(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            vec![video.to_string_lossy().to_string()],
            None,
            true,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn process_batch_empty_videos_errors() {
        let app = mock_app();
        let result = process_batch(
            app.handle().clone(),
            app.state::<ProcessingState>(),
            vec![],
            None,
            true,
        )
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn accepted_extensions_lists_the_documented_formats() {
        assert_eq!(
            accepted_extensions(),
            vec!["mp4", "mov", "mkv", "avi", "m4v"]
        );
    }
}
