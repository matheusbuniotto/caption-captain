use std::fs;
use std::process::Command;

/// End-to-end: `capcap run` on a real spoken-word video produces a
/// readable `.srt` sidecar with no `--lang` flag needed.
#[test]
fn run_produces_srt_sidecar_for_spoken_word_video() {
    let workdir = tempfile::tempdir().unwrap();
    let video_path = workdir.path().join("spoken-word.mp4");
    fs::copy("tests/fixtures/spoken-word.mp4", &video_path).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_capcap"))
        .arg("run")
        .arg(&video_path)
        .status()
        .expect("failed to run capcap binary");
    assert!(status.success());

    let srt_path = video_path.with_extension("srt");
    let srt = fs::read_to_string(&srt_path).expect("expected .srt sidecar next to input video");

    assert!(srt.contains("-->"), "missing SRT timestamp arrow:\n{srt}");
    assert!(
        srt.to_lowercase().contains("caption"),
        "expected recognizable speech in transcript:\n{srt}"
    );
}
