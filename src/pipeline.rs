//! The single orchestration path for "process this video": extract audio,
//! transcribe, write the `.srt` sidecar, and embed captions where the
//! container supports it. Both `main.rs` (CLI) and the Tauri GUI backend
//! call `process_video` and report progress through their own callback,
//! so there is exactly one place that knows the pipeline's shape.

use crate::{audio, mux, srt, transcribe};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Caller-supplied knobs for a single `process_video` run.
/// `translate_to` (Milestone 3) is intentionally not here yet.
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    /// ISO 639-1 language code to force; `None` lets Whisper auto-detect.
    pub lang: Option<String>,
    /// Skip muxing captions into the container; only write the `.srt`.
    pub no_embed: bool,
}

/// A stage the pipeline has entered, for progress reporting. The CLI prints
/// these; the GUI turns them into a progress bar / banner.
#[derive(Debug, Clone)]
pub enum PipelineStage {
    ExtractingAudio,
    Transcribing,
    /// Fired once the `.srt` sidecar has been written to `srt_path`.
    WritingSidecar {
        srt_path: PathBuf,
    },
    Embedding,
    /// The container doesn't support embedded captions (e.g. `.avi`); only
    /// the sidecar was written. `reason` is human-readable and container-
    /// agnostic enough to show as a GUI banner instead of stderr.
    SkippedEmbed {
        reason: String,
    },
    Done {
        srt_path: PathBuf,
        muxed_path: Option<PathBuf>,
    },
}

/// Everything a caller needs once the pipeline finishes successfully.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub srt_path: PathBuf,
    pub muxed_path: Option<PathBuf>,
    /// Set when embedding was skipped because the container doesn't
    /// support it (mirrors the last `SkippedEmbed` stage, if any).
    pub warning: Option<String>,
}

/// Runs the full pipeline against `video_path`, reporting progress through
/// `on_stage` as each step starts (or, for `SkippedEmbed`/`Done`, completes).
pub fn process_video(
    video_path: &Path,
    options: &PipelineOptions,
    mut on_stage: impl FnMut(PipelineStage),
) -> Result<PipelineOutput> {
    anyhow::ensure!(
        video_path.is_file(),
        "no such video file: {}",
        video_path.display()
    );

    let srt_path = video_path.with_extension("srt");
    let workdir = tempfile::tempdir()?;

    on_stage(PipelineStage::ExtractingAudio);
    let wav_path = audio::extract_wav(video_path, workdir.path())?;

    on_stage(PipelineStage::Transcribing);
    let cues = transcribe::transcribe(&wav_path, options.lang.as_deref())?;

    std::fs::write(&srt_path, srt::format_srt(&cues))?;
    on_stage(PipelineStage::WritingSidecar {
        srt_path: srt_path.clone(),
    });

    let mut muxed_path = None;
    let mut warning = None;

    if !options.no_embed {
        match mux::mux_strategy(video_path) {
            mux::MuxStrategy::SidecarOnly => {
                let ext = video_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("(no extension)");
                let reason = format!(
                    "embedding captions isn't reliable/supported for .{ext} files; \
                     wrote only the .srt sidecar."
                );
                on_stage(PipelineStage::SkippedEmbed {
                    reason: reason.clone(),
                });
                warning = Some(reason);
            }
            embeddable_strategy => {
                on_stage(PipelineStage::Embedding);
                muxed_path = Some(mux::embed_captions(
                    video_path,
                    &srt_path,
                    embeddable_strategy,
                )?);
            }
        }
    }

    on_stage(PipelineStage::Done {
        srt_path: srt_path.clone(),
        muxed_path: muxed_path.clone(),
    });

    Ok(PipelineOutput {
        srt_path,
        muxed_path,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn copy_fixture(workdir: &Path, fixture: &str, name: &str) -> PathBuf {
        let dest = workdir.join(name);
        fs::copy(format!("tests/fixtures/{fixture}"), &dest).unwrap();
        dest
    }

    #[test]
    fn reports_stages_in_order_and_returns_srt_and_muxed_paths() {
        let workdir = tempfile::tempdir().unwrap();
        let video = copy_fixture(workdir.path(), "spoken-word.mp4", "spoken-word.mp4");

        let mut stages = Vec::new();
        let output = process_video(&video, &PipelineOptions::default(), |stage| {
            stages.push(format!("{stage:?}"));
        })
        .expect("pipeline should succeed on a real fixture video");

        assert!(output.srt_path.is_file());
        assert!(output.muxed_path.as_deref().is_some_and(Path::is_file));
        assert!(output.warning.is_none());

        assert_eq!(
            stages
                .iter()
                .map(|s| s.split(&['(', ' '][..]).next().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "ExtractingAudio",
                "Transcribing",
                "WritingSidecar",
                "Embedding",
                "Done",
            ]
        );
    }

    #[test]
    fn no_embed_skips_muxing_and_reports_no_warning() {
        let workdir = tempfile::tempdir().unwrap();
        let video = copy_fixture(workdir.path(), "spoken-word.mp4", "spoken-word.mp4");

        let options = PipelineOptions {
            no_embed: true,
            ..Default::default()
        };
        let output = process_video(&video, &options, |_| {}).unwrap();

        assert!(output.srt_path.is_file());
        assert!(output.muxed_path.is_none());
        assert!(output.warning.is_none());
    }

    #[test]
    fn unsupported_container_reports_skipped_embed_warning() {
        let workdir = tempfile::tempdir().unwrap();
        let video = copy_fixture(workdir.path(), "spoken-word.avi", "spoken-word.avi");

        let mut saw_skip_reason = None;
        let output = process_video(&video, &PipelineOptions::default(), |stage| {
            if let PipelineStage::SkippedEmbed { reason } = stage {
                saw_skip_reason = Some(reason);
            }
        })
        .unwrap();

        assert!(output.muxed_path.is_none());
        let warning = output.warning.expect("expected a sidecar-only warning");
        assert!(warning.contains("avi"));
        assert_eq!(saw_skip_reason, Some(warning));
    }
}
