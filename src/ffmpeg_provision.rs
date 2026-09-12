//! `audio::extract_wav` and `mux::embed_captions` each need the bundled
//! ffmpeg binary present before they can spawn it. `ffmpeg_sidecar`'s own
//! `auto_download` isn't safe to call concurrently: on a cold cache, two
//! callers unpacking the same destination file at once can corrupt it
//! (seen as flaky "Failed to unpack ffmpeg" / "Failed to provision" test
//! failures in CI). Two kinds of concurrency need covering:
//! - Multiple threads in one process (e.g. `cargo test`'s unit tests) --
//!   guarded by the `OnceLock` below.
//! - Multiple separate `capcap` processes (e.g. `tests/run_transcribe.rs`
//!   spawning several subprocesses in parallel, all sharing the same
//!   ffmpeg-sidecar destination next to the binary) -- guarded by a lock
//!   file, since a `OnceLock` only helps within one process.

use anyhow::Context;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::sync::OnceLock;
use std::time::Duration;

static FFMPEG_READY: OnceLock<Result<(), String>> = OnceLock::new();

pub(crate) fn ensure_ffmpeg() -> anyhow::Result<()> {
    FFMPEG_READY
        .get_or_init(|| provision_across_processes().map_err(|e| e.to_string()))
        .clone()
        .map_err(|e| anyhow::anyhow!("failed to provision bundled ffmpeg: {e}"))
}

/// Acquires a lock file next to where ffmpeg-sidecar places its binary
/// before downloading, so concurrent `capcap` processes don't unpack the
/// archive on top of each other. Whoever creates the lock file downloads;
/// everyone else polls until ffmpeg is installed (or the lock is gone,
/// meaning the download failed and this caller should retry itself).
fn provision_across_processes() -> anyhow::Result<()> {
    if ffmpeg_sidecar::command::ffmpeg_is_installed() {
        return Ok(());
    }

    let lock_path = ffmpeg_sidecar::paths::sidecar_dir()
        .context("failed to determine ffmpeg sidecar directory")?
        .join(".ffmpeg-provision.lock");

    // Bounded to a generous 2 minutes so a crashed lock-holder can't wedge
    // every other caller forever; after that we just try the download
    // ourselves.
    for _ in 0..600 {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_lock_file) => {
                let result =
                    ffmpeg_sidecar::download::auto_download().map_err(|e| anyhow::anyhow!("{e}"));
                let _ = std::fs::remove_file(&lock_path);
                return result;
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                if ffmpeg_sidecar::command::ffmpeg_is_installed() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(e).context("failed to create ffmpeg provisioning lock file"),
        }
    }

    ffmpeg_sidecar::download::auto_download().map_err(|e| anyhow::anyhow!("{e}"))
}
