use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { video, .. } => {
            println!("capcap run: not yet implemented ({video})");
        }
        Command::Watch { folder, .. } => {
            println!("capcap watch: not yet implemented ({folder})");
        }
        Command::Gui => {
            println!("capcap gui: not yet implemented");
        }
    }
}
