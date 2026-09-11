use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::path::{Path, PathBuf};

/// Whether `video_path`'s container supports a `mov_text` subtitle track.
pub fn is_embeddable(video_path: &Path) -> bool {
    matches!(
        video_path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("mp4") | Some("mov")
    )
}

/// Remuxes `video_path` with `srt_path` burned in as a native `mov_text`
/// track. Video/audio streams are stream-copied, so this is a fast remux,
/// not a re-encode. The original file is never touched; the result is
/// written next to it as `<stem>.captioned.<ext>`.
pub fn embed_captions(video_path: &Path, srt_path: &Path) -> Result<PathBuf> {
    ffmpeg_sidecar::download::auto_download()
        .map_err(|e| anyhow::anyhow!("failed to provision bundled ffmpeg: {e}"))?;

    let out_path = captioned_output_path(video_path);

    let status = FfmpegCommand::new()
        .input(video_path.to_string_lossy())
        .input(srt_path.to_string_lossy())
        .args([
            "-map", "0:v", "-map", "0:a", "-map", "1:s:0", "-c", "copy", "-c:s", "mov_text",
        ])
        .overwrite()
        .output(out_path.to_string_lossy())
        .spawn()
        .context("failed to spawn bundled ffmpeg for muxing")?
        .wait()
        .context("bundled ffmpeg mux process failed")?;

    anyhow::ensure!(status.success(), "ffmpeg exited with {status}");
    Ok(out_path)
}

fn captioned_output_path(video_path: &Path) -> PathBuf {
    let ext = video_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    video_path.with_file_name(format!("{stem}.captioned.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_mp4_and_mov_as_embeddable() {
        assert!(is_embeddable(Path::new("video.mp4")));
        assert!(is_embeddable(Path::new("video.MOV")));
        assert!(!is_embeddable(Path::new("video.mkv")));
        assert!(!is_embeddable(Path::new("video")));
    }

    #[test]
    fn captioned_output_path_keeps_extension_and_adds_suffix() {
        assert_eq!(
            captioned_output_path(Path::new("/tmp/clips/video.mp4")),
            PathBuf::from("/tmp/clips/video.captioned.mp4")
        );
        assert_eq!(
            captioned_output_path(Path::new("clip.MOV")),
            PathBuf::from("clip.captioned.MOV")
        );
    }
}
