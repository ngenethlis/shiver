//! The `shiver` command: show a line from a voice in the terminal or as a
//! desktop notification.

mod init;
mod notify;
mod output;
mod sound;
mod stdout;
mod timer;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Args, CommandFactory, Parser, Subcommand};
use rand::SeedableRng;
use rand::rngs::StdRng;
use shiver_core::{Chorus, Filter, Interjection};

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
    /// Install the default pack into the packs folder.
    Init {
        /// Overwrite the default pack if it's already installed.
        #[arg(long)]
        force: bool,
    },
    /// Random popups through the day with a systemd user timer.
    Timer {
        #[command(subcommand)]
        action: TimerAction,
    },
}

#[derive(Subcommand)]
enum TimerAction {
    /// Install the timer and start it.
    Enable(EnableArgs),
    /// Stop the timer and remove it.
    Disable,
    /// Show whether the timer runs, its schedule and the next popup.
    Status,
}

#[derive(Args)]
struct EnableArgs {
    /// Time between popups, e.g. 30m, 1h, 1h30m.
    #[arg(long, default_value = "45m", value_parser = timer::parse_duration)]
    every: u32,
    /// Daily window for popups; may cross midnight (22:00-02:00).
    #[arg(long, default_value = "09:00-21:00", value_name = "HH:MM-HH:MM", value_parser = timer::parse_window)]
    between: (u32, u32),
    /// Each popup comes at a random point up to this long after its slot.
    #[arg(long, default_value = "20m", value_parser = timer::parse_duration)]
    jitter: u32,
    /// Replace unit files that weren't written by shiver.
    #[arg(long)]
    force: bool,
}

/// Added to "no packs found" messages.
const INIT_HINT: &str = "run `shiver init` to install the default pack";

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
    let explicit_dir = cli.packs_dir.or(env_dir.map(PathBuf::from));
    let dir = match explicit_dir.clone() {
        Some(dir) => dir,
        None => dirs::config_dir()
            .context("can't find the config folder; use --packs-dir")?
            .join("shiver")
            .join("packs"),
    };

    // `init` runs before loading, so broken packs can't stop it.
    if let Command::Init { force } = cli.command {
        let path = match init::install(&dir, force)? {
            init::Outcome::Created(path) => format!("created {}", path.display()),
            init::Outcome::Replaced(path) => format!("replaced {}", path.display()),
        };
        println!("{path}\n{}", init::HINT);
        return Ok(());
    }

    // `timer` doesn't need the packs either.
    if let Command::Timer { action } = cli.command {
        return run_timer(action, explicit_dir.as_deref());
    }

    let (chorus, warnings) = Chorus::load(&dir)?;
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
            let line = pick(&chorus, &dir, &filter, &mut rng)?;
            stdout::Stdout::detect().emit(&line)
        }
        Command::Notify(args) => {
            let line = match args.text {
                Some(text) => {
                    let voice = args.voice.expect("clap requires --voice with --text");
                    notify::one_off(&chorus, &voice, &text, args.sound)
                }
                None => {
                    let filter = Filter {
                        voice: args.voice,
                        tag: args.tag,
                    };
                    pick(&chorus, &dir, &filter, &mut rng)?
                }
            };
            notify::Notify.emit(&line)?;
            if !args.no_sound {
                sound::Sound.emit(&line)?;
            }
            Ok(())
        }
        Command::List => {
            print!("{}", list(&chorus, &dir));
            Ok(())
        }
        Command::Init { .. } | Command::Timer { .. } => {
            unreachable!("handled before loading the chorus")
        }
    }
}

fn run_timer(action: TimerAction, packs_dir: Option<&Path>) -> anyhow::Result<()> {
    let units_dir = timer::units_dir()?;
    let mut systemctl = timer::systemctl;
    let lines = match action {
        TimerAction::Enable(args) => {
            let schedule = timer::Schedule::new(args.every, args.between, args.jitter)
                .unwrap_or_else(|msg| {
                    Cli::command()
                        .error(clap::error::ErrorKind::ValueValidation, msg)
                        .exit()
                });
            let exe = std::env::current_exe()
                .and_then(|p| p.canonicalize())
                .context("can't find the shiver binary's path")?;
            let packs_dir = packs_dir
                .map(std::path::absolute)
                .transpose()
                .context("can't resolve the packs folder")?;
            let opts = timer::Enable {
                units_dir: &units_dir,
                exe: &exe,
                packs_dir: packs_dir.as_deref(),
                schedule,
                force: args.force,
            };
            timer::enable(&opts, &mut systemctl)?
        }
        TimerAction::Disable => timer::disable(&units_dir, &mut systemctl)?,
        TimerAction::Status => timer::status(&units_dir, &mut systemctl)?,
    };
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

fn pick(
    chorus: &Chorus,
    dir: &Path,
    filter: &Filter,
    rng: &mut StdRng,
) -> anyhow::Result<Interjection> {
    if chorus.packs().is_empty() {
        bail!("no packs found in {}; {INIT_HINT}", dir.display());
    }
    chorus.pick(filter, rng).context("no matching lines")
}

fn list(chorus: &Chorus, dir: &Path) -> String {
    if chorus.packs().is_empty() {
        return format!("no packs found in {}\n{INIT_HINT}\n", dir.display());
    }
    let blocks: Vec<String> = chorus
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
