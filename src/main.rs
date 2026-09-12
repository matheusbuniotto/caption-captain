use capcap::pipeline::{self, PipelineOptions, PipelineStage};
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::ExitCode;

/// Captain Caption: zero-cloud local video captioning, translation & muxing.
#[derive(Parser)]
#[command(name = "capcap", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Transcribe (and optionally translate) one or more videos in batch.
    Run {
        #[arg(required = true, num_args = 1..)]
        videos: Vec<String>,
        #[arg(long)]
        lang: Option<String>,
        #[arg(long = "translate-to")]
        translate_to: Option<String>,
        /// Skip muxing captions into the video container; only write the .srt sidecar.
        #[arg(long = "no-embed")]
        no_embed: bool,
    },
    /// Watch a folder and process videos as they arrive.
    Watch {
        folder: String,
        #[arg(long = "translate-to")]
        translate_to: Option<String>,
    },
    /// Launch the drag-and-drop desktop GUI.
    Gui,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Run {
            videos,
            lang,
            translate_to,
            no_embed,
        } => run(&videos, lang.as_deref(), translate_to.as_deref(), no_embed),
        Command::Watch { folder, .. } => {
            println!("capcap watch: not yet implemented ({folder})");
            Ok(())
        }
        Command::Gui => launch_gui(),
    };

    if let Err(err) = result {
        eprintln!("error: {err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// CLI wrapper around the shared pipeline: turns each `PipelineStage`
/// into the same `println!`/`eprintln!` calls the CLI has always made, so
/// `tests/run_transcribe.rs` sees byte-for-byte unchanged output for single
/// videos, and sequential progress for batch processing.
fn run(
    videos: &[String],
    lang: Option<&str>,
    translate_to: Option<&str>,
    no_embed: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        translate_to.is_none(),
        "--translate-to isn't implemented yet"
    );

    let is_batch = videos.len() > 1;
    let mut failed = 0;

    for (i, video) in videos.iter().enumerate() {
        let video_path = Path::new(video);
        if !video_path.is_file() {
            eprintln!("error: no such video file: {video}");
            failed += 1;
            continue;
        }

        if is_batch {
            println!("\n[{}/{}] Processing {}", i + 1, videos.len(), video);
        }

        let options = PipelineOptions {
            lang: lang.map(str::to_string),
            no_embed,
        };

        let res = pipeline::process_video(video_path, &options, |stage| match stage {
            PipelineStage::ExtractingAudio => println!("Extracting audio..."),
            PipelineStage::Transcribing => println!("Transcribing..."),
            PipelineStage::WritingSidecar { srt_path } => println!("Wrote {}", srt_path.display()),
            PipelineStage::Embedding => println!("Embedding captions..."),
            PipelineStage::SkippedEmbed { reason } => eprintln!("Warning: {reason}"),
            PipelineStage::Done {
                muxed_path,
                language,
                ..
            } => {
                println!("Detected language: {language}");
                if let Some(muxed_path) = muxed_path {
                    println!("Wrote {}", muxed_path.display());
                }
            }
        });

        if let Err(err) = res {
            eprintln!("error processing {video}: {err:#}");
            failed += 1;
        }
    }

    if failed > 0 {
        anyhow::bail!("{failed} of {} video(s) failed to process", videos.len());
    }

    Ok(())
}

/// `capcap gui` launches the Tauri desktop app rather than reimplementing
/// it here: the GUI is a separate binary (built by `gui/src-tauri`) so the
/// CLI crate never needs the Tauri/webview toolchain to build or test.
/// We look for it next to the running `capcap` binary first (that's where
/// packaged release bundles and local dev builds put it), then fall back
/// to PATH, and otherwise tell the user how to build/run it themselves.
fn launch_gui() -> anyhow::Result<()> {
    let gui_binary_name = if cfg!(windows) {
        "capcap-gui.exe"
    } else {
        "capcap-gui"
    };

    let candidate = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(gui_binary_name)))
        .filter(|path| path.is_file());

    let gui_path = match candidate {
        Some(path) => Some(path),
        None => which_on_path(gui_binary_name),
    };

    if let Some(path) = gui_path {
        std::process::Command::new(path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to launch capcap-gui: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        for app_path in [
            std::path::PathBuf::from("/Applications/Capcap.app"),
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join("Applications/Capcap.app"))
                .unwrap_or_default(),
        ] {
            if app_path.is_dir() {
                std::process::Command::new("open")
                    .arg(app_path)
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("failed to launch Capcap.app: {e}"))?;
                return Ok(());
            }
        }
    }

    println!(
        "capcap gui: no {gui_binary_name} found next to this binary or on PATH.\n\
         Build it with `npm run tauri -- build`, or run it in dev mode with `npm run dev`."
    );
    Ok(())
}

fn which_on_path(binary_name: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(binary_name))
        .find(|path| path.is_file())
}
