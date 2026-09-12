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

    let muxed_path = workdir.path().join("spoken-word.captioned.mp4");
    assert!(
        muxed_path.is_file(),
        "expected muxed captioned video at {}",
        muxed_path.display()
    );

    let input_size = fs::metadata("tests/fixtures/spoken-word.mp4")
        .unwrap()
        .len();
    let output_size = fs::metadata(&muxed_path).unwrap().len();
    let diff = input_size.abs_diff(output_size);
    assert!(
        diff < input_size / 10,
        "expected a stream-copy remux to be nearly the same size (input {input_size}, output {output_size})"
    );

    let original_size = fs::metadata(&video_path).unwrap().len();
    assert_eq!(
        original_size, input_size,
        "original input file must be untouched"
    );

    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=codec_name",
        ])
        .arg(&muxed_path)
        .output()
        .expect("failed to run ffprobe");
    let probe_out = String::from_utf8_lossy(&probe.stdout);
    assert!(
        probe_out.contains("mov_text"),
        "expected a mov_text subtitle stream in muxed output, got:\n{probe_out}"
    );
}

/// `--no-embed` skips muxing entirely, producing only the .srt sidecar.
#[test]
fn run_with_no_embed_skips_muxed_output() {
    let workdir = tempfile::tempdir().unwrap();
    let video_path = workdir.path().join("spoken-word.mp4");
    fs::copy("tests/fixtures/spoken-word.mp4", &video_path).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_capcap"))
        .arg("run")
        .arg(&video_path)
        .arg("--no-embed")
        .status()
        .expect("failed to run capcap binary");
    assert!(status.success());

    let srt_path = video_path.with_extension("srt");
    assert!(
        srt_path.is_file(),
        "expected .srt sidecar to still be produced"
    );

    let muxed_path = workdir.path().join("spoken-word.captioned.mp4");
    assert!(
        !muxed_path.is_file(),
        "expected no muxed video file with --no-embed"
    );
}

/// MKV input gets a native SRT subtitle stream, zero-recode, original
/// untouched.
#[test]
fn run_embeds_native_srt_for_mkv() {
    let workdir = tempfile::tempdir().unwrap();
    let video_path = workdir.path().join("spoken-word.mkv");
    fs::copy("tests/fixtures/spoken-word.mkv", &video_path).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_capcap"))
        .arg("run")
        .arg(&video_path)
        .status()
        .expect("failed to run capcap binary");
    assert!(status.success());

    let srt_path = video_path.with_extension("srt");
    assert!(srt_path.is_file(), "expected .srt sidecar to be produced");

    let muxed_path = workdir.path().join("spoken-word.captioned.mkv");
    assert!(
        muxed_path.is_file(),
        "expected muxed captioned video at {}",
        muxed_path.display()
    );

    let input_size = fs::metadata("tests/fixtures/spoken-word.mkv")
        .unwrap()
        .len();
    let output_size = fs::metadata(&muxed_path).unwrap().len();
    let diff = input_size.abs_diff(output_size);
    assert!(
        diff < input_size / 10,
        "expected a stream-copy remux to be nearly the same size (input {input_size}, output {output_size})"
    );

    let original_size = fs::metadata(&video_path).unwrap().len();
    assert_eq!(
        original_size, input_size,
        "original input file must be untouched"
    );

    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=codec_name",
        ])
        .arg(&muxed_path)
        .output()
        .expect("failed to run ffprobe");
    let probe_out = String::from_utf8_lossy(&probe.stdout);
    assert!(
        probe_out.contains("subrip"),
        "expected a native SRT subtitle stream in muxed output, got:\n{probe_out}"
    );
}

/// Unsupported containers (e.g. AVI) get only the .srt sidecar, no muxed
/// video, and a warning explaining why.
#[test]
fn run_skips_embed_for_unsupported_container() {
    let workdir = tempfile::tempdir().unwrap();
    let video_path = workdir.path().join("spoken-word.avi");
    fs::copy("tests/fixtures/spoken-word.avi", &video_path).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_capcap"))
        .arg("run")
        .arg(&video_path)
        .output()
        .expect("failed to run capcap binary");
    assert!(output.status.success());

    let srt_path = video_path.with_extension("srt");
    assert!(srt_path.is_file(), "expected .srt sidecar to be produced");

    let muxed_path = workdir.path().join("spoken-word.captioned.avi");
    assert!(
        !muxed_path.is_file(),
        "expected no muxed video file for an unsupported container"
    );

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("warning") && stderr.contains("avi"),
        "expected a warning naming the unsupported container:\n{stderr}"
    );
}

/// Batch run: passing multiple videos processes each and produces outputs for all.
#[test]
fn run_processes_multiple_videos_in_batch() {
    let workdir = tempfile::tempdir().unwrap();
    let video1 = workdir.path().join("spoken-word-1.mp4");
    let video2 = workdir.path().join("spoken-word-2.mkv");
    fs::copy("tests/fixtures/spoken-word.mp4", &video1).unwrap();
    fs::copy("tests/fixtures/spoken-word.mkv", &video2).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_capcap"))
        .arg("run")
        .arg(&video1)
        .arg(&video2)
        .output()
        .expect("failed to run capcap binary in batch");
    assert!(output.status.success());

    assert!(video1.with_extension("srt").is_file());
    assert!(video2.with_extension("srt").is_file());
    assert!(workdir.path().join("spoken-word-1.captioned.mp4").is_file());
    assert!(workdir.path().join("spoken-word-2.captioned.mkv").is_file());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[1/2] Processing"));
    assert!(stdout.contains("[2/2] Processing"));
}
