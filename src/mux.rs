use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::path::{Path, PathBuf};

/// How captions should be attached to a video, based on its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuxStrategy {
    /// MP4/MOV: embed as a `mov_text` subtitle stream.
    MovText,
    /// MKV: embed as a native SRT subtitle stream.
    MkvSrt,
    /// Any other container: embedding isn't reliable/supported, so only
    /// the `.srt` sidecar is produced.
    SidecarOnly,
}

/// Decides how to attach captions to `video_path` based on its extension.
/// Pure and independent of ffmpeg: MP4/MOV -> mov_text, MKV -> native SRT,
/// anything else -> sidecar-only.
pub fn mux_strategy(video_path: &Path) -> MuxStrategy {
    match video_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("mp4") | Some("mov") => MuxStrategy::MovText,
        Some("mkv") => MuxStrategy::MkvSrt,
        _ => MuxStrategy::SidecarOnly,
    }
}

/// Remuxes `video_path` with `srt_path` embedded as a subtitle track per
/// `strategy`. Video/audio streams are stream-copied, so this is a fast
/// remux, not a re-encode. The original file is never touched; the result
/// is written next to it as `<stem>.captioned.<ext>`.
///
/// Returns an error if called with `MuxStrategy::SidecarOnly`; callers
/// should check the strategy before invoking ffmpeg at all.
pub fn embed_captions(
    video_path: &Path,
    srt_path: &Path,
    strategy: MuxStrategy,
) -> Result<PathBuf> {
    let subtitle_codec = match strategy {
        MuxStrategy::MovText => "mov_text",
        MuxStrategy::MkvSrt => "srt",
        MuxStrategy::SidecarOnly => {
            anyhow::bail!("MuxStrategy::SidecarOnly cannot be embedded")
        }
    };

    ffmpeg_sidecar::download::auto_download()
        .map_err(|e| anyhow::anyhow!("failed to provision bundled ffmpeg: {e}"))?;

    let out_path = captioned_output_path(video_path);

    let status = FfmpegCommand::new()
        .input(video_path.to_string_lossy())
        .input(srt_path.to_string_lossy())
        .args([
            "-map",
            "0:v",
            "-map",
            "0:a",
            "-map",
            "1:s:0",
            "-c",
            "copy",
            "-c:s",
            subtitle_codec,
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
    fn mp4_and_mov_use_mov_text() {
        assert_eq!(mux_strategy(Path::new("video.mp4")), MuxStrategy::MovText);
        assert_eq!(mux_strategy(Path::new("video.MOV")), MuxStrategy::MovText);
    }

    #[test]
    fn mkv_uses_native_srt() {
        assert_eq!(mux_strategy(Path::new("video.mkv")), MuxStrategy::MkvSrt);
        assert_eq!(mux_strategy(Path::new("video.MKV")), MuxStrategy::MkvSrt);
    }

    #[test]
    fn unsupported_containers_are_sidecar_only() {
        assert_eq!(
            mux_strategy(Path::new("video.avi")),
            MuxStrategy::SidecarOnly
        );
        assert_eq!(mux_strategy(Path::new("video")), MuxStrategy::SidecarOnly);
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
