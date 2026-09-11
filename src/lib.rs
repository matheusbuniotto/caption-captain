//! Shared library for capcap: the CLI binary (`src/main.rs`) and the Tauri
//! desktop app (`gui/src-tauri`) both link this crate so there is exactly
//! one place that orchestrates "process this video" (see `pipeline`).

pub mod audio;
pub mod mux;
pub mod pipeline;
pub mod srt;
pub mod transcribe;
