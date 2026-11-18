//! The `shiver` command: show a line from a voice in the terminal or as a
//! desktop notification.

mod config;
mod history;
mod hook;
mod icon;
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

    /// Config file [env: SHIVER_CONFIG] [default: ~/.config/shiver/config.toml]
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

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
    /// Print a shell snippet that lets a voice react to failed commands.
    ///
    /// Add `eval "$(shiver hook zsh)"` to ~/.zshrc (or `bash`, to ~/.bashrc).
    Hook(HookArgs),
    /// Called by the shell hook after a failed command; may show a `fail` line.
    OnFail(OnFailArgs),
    /// Show the config file's location and every setting in effect.
    Config,
}

#[derive(Args)]
struct HookArgs {
    /// The shell to print the snippet for.
    shell: hook::Shell,
    /// At most one popup per this long [default: 10m].
    #[arg(long, value_parser = timer::parse_duration)]
    min_gap: Option<u32>,
    /// Popups from the hook are silent.
    #[arg(long)]
    no_sound: bool,
}

#[derive(Args)]
struct OnFailArgs {
    /// The failed command's exit code.
    #[arg(long, allow_hyphen_values = true)]
    exit_code: i32,
    /// The command line that failed.
    #[arg(long, allow_hyphen_values = true)]
    command: Option<String>,
    /// At most one popup per this long [default: hook.min_gap, or 10m].
    #[arg(long, value_parser = timer::parse_duration)]
    min_gap: Option<u32>,
    /// Don't play a sound.
    #[arg(long)]
    no_sound: bool,
    /// Print the decision instead of notifying; changes no state.
    #[arg(long)]
    dry_run: bool,
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
    /// Pretend it's this time of day (for `morning` and `late` lines).
    #[arg(long, value_name = "HH:MM", value_parser = parse_clock)]
    at: Option<u32>,
}

#[derive(Args)]
struct NotifyArgs {
    /// Only lines from this voice (with --text: the voice to show).
    #[arg(long)]
    voice: Option<String>,
    /// Only lines with this tag.
    #[arg(long, conflicts_with = "text")]
    tag: Option<String>,
    /// Pretend it's this time of day (for `morning` and `late` lines).
    #[arg(long, value_name = "HH:MM", value_parser = parse_clock, conflicts_with = "text")]
    at: Option<u32>,
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
    let (config_path, explicit_config) = config::locate(cli.config)?;
    let (cfg, config_exists) = config::load(&config_path, explicit_config)?;

    // --packs-dir, then SHIVER_PACKS_DIR (empty counts as unset), then the
    // config file, then the default.
    let env_dir = std::env::var_os("SHIVER_PACKS_DIR").filter(|v| !v.is_empty());
    let explicit_dir = cli.packs_dir.or(env_dir.map(PathBuf::from));
    let dir = match explicit_dir.clone().or(cfg.packs_dir.clone()) {
        Some(dir) => dir,
        None => dirs::config_dir()
            .context("can't find the config folder; use --packs-dir")?
            .join("shiver")
            .join("packs"),
    };

    if let Command::Config = cli.command {
        let state = if config_exists {
            "exists"
        } else {
            "not found; using defaults"
        };
        println!("# {} ({state})", config_path.display());
        print!("{}", cfg.to_toml(&dir));
        return Ok(());
    }

    // `init` runs before loading, so broken packs can't stop it.
    if let Command::Init { force } = cli.command {
        let path = match init::install(&dir, force)? {
            init::Outcome::Created(path) => format!("created {}", path.display()),
            init::Outcome::Replaced(path) => format!("replaced {}", path.display()),
        };
        println!("{path}\n{}", init::HINT);
        return Ok(());
    }

    // `timer` and `hook` don't need the packs either.
    let command = match cli.command {
        Command::Timer { action } => return run_timer(action, explicit_dir.as_deref()),
        Command::Hook(args) => {
            let exe = std::env::current_exe()
                .and_then(|p| p.canonicalize())
                .context("can't find the shiver binary's path")?;
            print!(
                "{}",
                hook::snippet(args.shell, &exe, args.min_gap, args.no_sound)
            );
            return Ok(());
        }
        Command::OnFail(args) => return on_fail(&args, &dir, &cfg),
        command => command,
    };

    let (chorus, warnings) = Chorus::load_skipping(&dir, &cfg.disabled_packs)?;
    for warning in &warnings {
        output::warn(warning);
    }
    let mut rng = match cli.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => rand::make_rng(),
    };

    match command {
        Command::Say(args) => {
            let filter = Filter {
                voice: args.voice,
                tag: args.tag,
                context: context(args.at, &cfg),
            };
            let recent = Recent::load(&cfg);
            let line = pick(&chorus, &dir, &filter, &recent, &mut rng)?;
            stdout::Stdout::detect().emit(&line)?;
            recent.record(&line);
            Ok(())
        }
        Command::Notify(args) => {
            // One-off messages don't touch the history.
            let (line, recent) = match args.text {
                Some(text) => {
                    let voice = args.voice.expect("clap requires --voice with --text");
                    (notify::one_off(&chorus, &voice, &text, args.sound), None)
                }
                None => {
                    let filter = Filter {
                        voice: args.voice,
                        tag: args.tag,
                        context: context(args.at, &cfg),
                    };
                    let recent = Recent::load(&cfg);
                    let line = pick(&chorus, &dir, &filter, &recent, &mut rng)?;
                    (line, Some(recent))
                }
            };
            notify::Notify {
                default_icon: cfg.default_icon,
            }
            .emit(&line)?;
            if let Some(recent) = recent {
                recent.record(&line);
            }
            if !args.no_sound && cfg.sound_enabled {
                sound::Sound { volume: cfg.volume }.emit(&line)?;
            }
            Ok(())
        }
        Command::List => {
            print!("{}", list(&chorus, &dir));
            Ok(())
        }
        Command::Init { .. }
        | Command::Timer { .. }
        | Command::Hook(_)
        | Command::OnFail(_)
        | Command::Config => {
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

/// `shiver on-fail`: maybe show a `fail` line. Quiet unless `--dry-run`.
fn on_fail(args: &OnFailArgs, dir: &Path, cfg: &config::Config) -> anyhow::Result<()> {
    let say = |msg: String| {
        if args.dry_run {
            println!("{msg}");
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let last_path = hook::last_fail_path();
    let last = last_path.as_deref().and_then(hook::read_last_fail);
    if let Some(reason) = hook::skip_reason(
        args.exit_code,
        args.command.as_deref(),
        &cfg.ignore_commands,
        last,
        now,
        args.min_gap.unwrap_or(cfg.min_gap),
    ) {
        say(format!("skip: {reason}"));
        return Ok(());
    }

    // Pack warnings are left to `say` and `list`; the hook stays quiet.
    let Ok((chorus, _warnings)) = Chorus::load_skipping(dir, &cfg.disabled_packs) else {
        say("skip: no fail lines".into());
        return Ok(());
    };
    let filter = Filter {
        voice: None,
        tag: Some("fail".into()),
        context: context(None, cfg),
    };
    let recent = Recent::load(cfg);
    let Some(line) = chorus.pick_with_history(
        &filter,
        &history::seen(&recent.entries),
        &mut rand::make_rng::<StdRng>(),
    ) else {
        say("skip: no fail lines".into());
        return Ok(());
    };
    if args.dry_run {
        say(format!(
            "would notify: {}: {}",
            line.voice.display, line.text
        ));
        return Ok(());
    }

    notify::Notify {
        default_icon: cfg.default_icon,
    }
    .emit(&line)?;
    recent.record(&line);
    if let Some(path) = &last_path {
        hook::write_last_fail(path, now);
    }
    if !args.no_sound && cfg.sound_enabled {
        sound::Sound { volume: cfg.volume }.emit(&line)?;
    }
    Ok(())
}

/// Parses `HH:MM` (00:00 to 23:59) into minutes after midnight.
fn parse_clock(s: &str) -> Result<u32, String> {
    let err = || format!("invalid time `{s}` (use HH:MM, e.g. 23:30)");
    let (h, m) = s.split_once(':').ok_or_else(err)?;
    if h.len() != 2 || m.len() != 2 {
        return Err(err());
    }
    let (h, m): (u32, u32) = (h.parse().map_err(|_| err())?, m.parse().map_err(|_| err())?);
    if h < 24 && m < 60 {
        Ok(h * 60 + m)
    } else {
        Err(err())
    }
}

/// The time-of-day context: `--at` if given, else the local time.
fn context(at: Option<u32>, cfg: &config::Config) -> shiver_core::Context {
    if let Some(minutes) = at {
        return cfg.context.at(minutes);
    }
    match jiff::tz::TimeZone::try_system() {
        Ok(tz) => {
            let now = jiff::Timestamp::now().to_zoned_datetime(tz);
            let minutes = u32::try_from(now.hour()).unwrap_or(0) * 60
                + u32::try_from(now.minute()).unwrap_or(0);
            cfg.context.at(minutes)
        }
        Err(e) => {
            output::warn(format!("can't read the local time: {e}"));
            shiver_core::Context::none()
        }
    }
}

/// Recent picks, for cooldowns. Without a state folder, it's empty and
/// nothing is recorded.
struct Recent {
    path: Option<PathBuf>,
    entries: Vec<history::Entry>,
    keep: usize,
}

impl Recent {
    /// Loads the history, unless cooldowns are turned off in the config.
    fn load(cfg: &config::Config) -> Self {
        let path = cfg.cooldowns.then(history::path).flatten();
        let entries = path.as_deref().map(history::load).unwrap_or_default();
        Recent {
            path,
            entries,
            keep: cfg.history,
        }
    }

    fn record(self, line: &Interjection) {
        if let Some(path) = &self.path {
            history::record(path, self.entries, line, self.keep);
        }
    }
}

fn pick(
    chorus: &Chorus,
    dir: &Path,
    filter: &Filter,
    recent: &Recent,
    rng: &mut StdRng,
) -> anyhow::Result<Interjection> {
    if chorus.packs().is_empty() {
        bail!("no packs found in {}; {INIT_HINT}", dir.display());
    }
    chorus
        .pick_with_history(filter, &history::seen(&recent.entries), rng)
        .context("no matching lines")
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
