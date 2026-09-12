use crate::srt::Cue;
use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// The multilingual `tiny` Whisper model, embedded at compile time so
/// transcription needs no network access even on a first run.
static TINY_MODEL: &[u8] = include_bytes!(env!("CAPCAP_TINY_MODEL_PATH"));

/// Transcribes a 16kHz mono WAV file. Auto-detects the spoken language
/// unless `lang` (an ISO 639-1 code, e.g. "en") forces one. Returns the
/// cues alongside the language Whisper actually used (the forced `lang`,
/// or whatever it auto-detected), as an ISO 639-1 code.
pub fn transcribe(wav_path: &Path, lang: Option<&str>) -> Result<(Vec<Cue>, String)> {
    // whisper.cpp/GGML log straight to stdout/stderr by default; route them
    // into whisper-rs's hooks instead, which drops them since no log/tracing
    // backend is enabled, keeping CLI output clean.
    whisper_rs::install_logging_hooks();

    let samples = read_wav_mono_f32(wav_path)?;

    let ctx = WhisperContext::new_from_buffer_with_params(
        TINY_MODEL,
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow::anyhow!("failed to load bundled Whisper model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("failed to create Whisper inference state: {e}"))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(lang); // None => auto-detect
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, &samples)
        .map_err(|e| anyhow::anyhow!("Whisper inference failed: {e}"))?;

    let detected_language = lang.map(str::to_string).unwrap_or_else(|| {
        let lang_id = state.full_lang_id_from_state();
        whisper_rs::get_lang_str(lang_id)
            .unwrap_or("unknown")
            .to_string()
    });

    let mut cues = Vec::new();
    for segment in state.as_iter() {
        let text = segment
            .to_str_lossy()
            .map_err(|e| anyhow::anyhow!("failed to read segment text: {e}"))?
            .to_string();
        if text.trim().is_empty() {
            continue;
        }
        cues.push(Cue {
            start_ms: (segment.start_timestamp() * 10) as u32,
            end_ms: (segment.end_timestamp() * 10) as u32,
            text,
        });
    }
    Ok((cues, detected_language))
}

/// Reads a WAV file into mono f32 samples at whatever sample rate it was
/// recorded at. `audio::extract_wav` always produces 16kHz mono, which is
/// what Whisper expects, so no resampling is done here.
fn read_wav_mono_f32(wav_path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(wav_path).context("failed to open extracted WAV audio")?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.channels == 1 && spec.sample_rate == 16_000,
        "expected 16kHz mono WAV, got {}ch @ {}Hz",
        spec.channels,
        spec.sample_rate
    );

    let samples: Result<Vec<f32>, _> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect(),
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|s| s as f32 / i16::MAX as f32))
            .collect(),
    };
    samples.context("failed to read WAV samples")
}
