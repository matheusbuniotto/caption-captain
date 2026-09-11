mod audio;
mod mux;
mod srt;
mod transcribe;

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
        Command::Gui => {
            println!("capcap gui: not yet implemented");
            Ok(())
        }
    };

    if let Err(err) = result {
        eprintln!("error: {err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

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

    let srt_path = video_path.with_extension("srt");
    let workdir = tempfile::tempdir()?;

    println!("Extracting audio...");
    let wav_path = audio::extract_wav(video_path, workdir.path())?;

    println!("Transcribing...");
    let cues = transcribe::transcribe(&wav_path, lang)?;

    std::fs::write(&srt_path, srt::format_srt(&cues))?;
    println!("Wrote {}", srt_path.display());

    if !no_embed {
        match mux::mux_strategy(video_path) {
            mux::MuxStrategy::SidecarOnly => {
                let ext = video_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("(no extension)");
                eprintln!(
                    "Warning: embedding captions isn't reliable/supported for .{ext} files; \
                     wrote only the .srt sidecar."
                );
            }
            embeddable_strategy => {
                println!("Embedding captions...");
                let muxed_path = mux::embed_captions(video_path, &srt_path, embeddable_strategy)?;
                println!("Wrote {}", muxed_path.display());
            }
        }
    }

    Ok(())
}
