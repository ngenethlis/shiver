//! The `shiver` command: show a line from a voice in the terminal or as a
//! desktop notification.

mod notify;
mod output;
mod sound;
mod stdout;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
use rand::SeedableRng;
use rand::rngs::StdRng;
use shiver_core::{Filter, Interjection, Library};

use crate::output::Output;

#[derive(Parser)]
#[command(name = "shiver", version, about = "Inner-voice popups for the desktop")]
struct Cli {
    /// Folder containing packs [env: SHIVER_PACKS_DIR] [default: ~/.config/shiver/packs]
    #[arg(long, global = true, value_name = "DIR")]
    packs_dir: Option<PathBuf>,

    /// Seed for the random pick (for tests).
    #[arg(long, global = true, hide = true)]
    seed: Option<u64>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a random line in the terminal.
    Say(SayArgs),
    /// Show a random line (or a one-off message) as a desktop notification.
    Notify(NotifyArgs),
    /// List packs, their voices and line counts.
    List,
}

#[derive(Args)]
struct SayArgs {
    /// Only lines from this voice.
    #[arg(long)]
    voice: Option<String>,
    /// Only lines with this tag.
    #[arg(long)]
    tag: Option<String>,
}

#[derive(Args)]
struct NotifyArgs {
    /// Only lines from this voice (with --text: the voice to show).
    #[arg(long)]
    voice: Option<String>,
    /// Only lines with this tag.
    #[arg(long, conflicts_with = "text")]
    tag: Option<String>,
    /// Show this text instead of a random line.
    #[arg(long, requires = "voice")]
    text: Option<String>,
    /// Sound to play with --text, instead of the voice's.
    #[arg(long, requires = "text", value_name = "FILE")]
    sound: Option<PathBuf>,
    /// Don't play a sound.
    #[arg(long)]
    no_sound: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("shiver: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    // An empty SHIVER_PACKS_DIR counts as unset.
    let env_dir = std::env::var_os("SHIVER_PACKS_DIR").filter(|v| !v.is_empty());
    let dir = match cli.packs_dir.or(env_dir.map(PathBuf::from)) {
        Some(dir) => dir,
        None => dirs::config_dir()
            .context("can't find the config folder; use --packs-dir")?
            .join("shiver")
            .join("packs"),
    };
    let (library, warnings) = Library::load(&dir)?;
    for warning in &warnings {
        output::warn(warning);
    }
    let mut rng = match cli.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => rand::make_rng(),
    };

    match cli.command {
        Command::Say(args) => {
            let filter = Filter {
                voice: args.voice,
                tag: args.tag,
            };
            let line = pick(&library, &dir, &filter, &mut rng)?;
            stdout::Stdout::detect().emit(&line)
        }
        Command::Notify(args) => {
            let line = match args.text {
                Some(text) => {
                    let voice = args.voice.expect("clap requires --voice with --text");
                    notify::one_off(&library, &voice, &text, args.sound)
                }
                None => {
                    let filter = Filter {
                        voice: args.voice,
                        tag: args.tag,
                    };
                    pick(&library, &dir, &filter, &mut rng)?
                }
            };
            notify::Notify.emit(&line)?;
            if !args.no_sound {
                sound::Sound.emit(&line)?;
            }
            Ok(())
        }
        Command::List => {
            print!("{}", list(&library, &dir));
            Ok(())
        }
    }
}

fn pick(
    library: &Library,
    dir: &Path,
    filter: &Filter,
    rng: &mut StdRng,
) -> anyhow::Result<Interjection> {
    if library.packs().is_empty() {
        bail!("no packs found in {}", dir.display());
    }
    library.pick(filter, rng).context("no matching lines")
}

fn list(library: &Library, dir: &Path) -> String {
    if library.packs().is_empty() {
        return format!("no packs found in {}\n", dir.display());
    }
    let blocks: Vec<String> = library
        .packs()
        .iter()
        .map(|p| {
            let voices = if p.voices.is_empty() {
                "none".to_owned()
            } else {
                p.voices.join(", ")
            };
            format!(
                "{} ({}, {})\n  {}\n  voices: {voices}\n",
                p.name,
                plural(p.voices.len(), "voice"),
                plural(p.lines, "line"),
                p.dir.display(),
            )
        })
        .collect();
    blocks.join("\n")
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}
