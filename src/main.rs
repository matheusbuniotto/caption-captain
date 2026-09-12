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
    /// Transcribe (and optionally translate) a single video.
    Run {
        video: String,
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
            video,
            lang,
            translate_to,
            no_embed,
        } => run(&video, lang.as_deref(), translate_to.as_deref(), no_embed),
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

/// Thin CLI wrapper around the shared pipeline: turns each `PipelineStage`
/// into the same `println!`/`eprintln!` calls the CLI has always made, so
/// `tests/run_transcribe.rs` sees byte-for-byte unchanged output.
fn run(
    video: &str,
    lang: Option<&str>,
    translate_to: Option<&str>,
    no_embed: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        translate_to.is_none(),
        "--translate-to isn't implemented yet"
    );

    let video_path = Path::new(video);
    anyhow::ensure!(video_path.is_file(), "no such video file: {video}");

    let options = PipelineOptions {
        lang: lang.map(str::to_string),
        no_embed,
    };

    pipeline::process_video(video_path, &options, |stage| match stage {
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
    })
    .map(|_| ())
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

    match gui_path {
        Some(path) => {
            std::process::Command::new(path)
                .spawn()
                .map_err(|e| anyhow::anyhow!("failed to launch capcap-gui: {e}"))?;
            Ok(())
        }
        None => {
            println!(
                "capcap gui: no {gui_binary_name} found next to this binary or on PATH.\n\
                 Build it with `cd gui && npm install && npm run tauri build`, or run it in \
                 dev mode with `cd gui && npm install && npx tauri dev`."
            );
            Ok(())
        }
    }
}

fn which_on_path(binary_name: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(binary_name))
        .find(|path| path.is_file())
}
