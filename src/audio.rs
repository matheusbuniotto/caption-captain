use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::path::{Path, PathBuf};

/// Extracts a 16kHz mono WAV from `video_path` into a sibling temp file,
/// using the managed ffmpeg binary (no system ffmpeg install required).
pub fn extract_wav(video_path: &Path, out_dir: &Path) -> Result<PathBuf> {
    ffmpeg_sidecar::download::auto_download()
        .map_err(|e| anyhow::anyhow!("failed to provision bundled ffmpeg: {e}"))?;

    let wav_path = out_dir.join("capcap-audio.wav");

    let status = FfmpegCommand::new()
        .input(video_path.to_string_lossy())
        .args(["-vn", "-ar", "16000", "-ac", "1", "-f", "wav"])
        .overwrite()
        .output(wav_path.to_string_lossy())
        .spawn()
        .context("failed to spawn bundled ffmpeg")?
        .wait()
        .context("bundled ffmpeg process failed")?;

    anyhow::ensure!(status.success(), "ffmpeg exited with {status}");
    Ok(wav_path)
}
