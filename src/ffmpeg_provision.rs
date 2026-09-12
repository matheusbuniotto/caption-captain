//! `audio::extract_wav` and `mux::embed_captions` each need the bundled
//! ffmpeg binary present before they can spawn it. `ffmpeg_sidecar`'s own
//! `auto_download` isn't safe to call concurrently: on a cold cache, two
//! threads unpacking the same destination file at once can corrupt it
//! (seen as flaky "Failed to unpack ffmpeg" test failures in CI). Provision
//! it exactly once per process instead.

use std::sync::OnceLock;

static FFMPEG_READY: OnceLock<Result<(), String>> = OnceLock::new();

pub(crate) fn ensure_ffmpeg() -> anyhow::Result<()> {
    FFMPEG_READY
        .get_or_init(|| ffmpeg_sidecar::download::auto_download().map_err(|e| e.to_string()))
        .clone()
        .map_err(|e| anyhow::anyhow!("failed to provision bundled ffmpeg: {e}"))
}
