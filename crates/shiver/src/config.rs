//! The optional config file, `~/.config/shiver/config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use serde::Deserialize;
use shiver_core::ContextRules;

use crate::timer::{format_duration, parse_duration, parse_window};

/// Every setting, validated, with defaults filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub packs_dir: Option<PathBuf>,
    pub disabled_packs: Vec<String>,
    pub sound_enabled: bool,
    pub volume: f64,
    pub default_icon: bool,
    pub cooldowns: bool,
    pub history: usize,
    pub context: ContextRules,
    pub min_gap: u32,
    pub ignore_commands: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            packs_dir: None,
            disabled_packs: Vec::new(),
            sound_enabled: true,
            volume: 1.0,
            default_icon: true,
            cooldowns: true,
            history: 50,
            context: ContextRules::default(),
            min_gap: 10,
            ignore_commands: Vec::new(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct Raw {
    packs_dir: Option<String>,
    disabled_packs: Option<Vec<String>>,
    sound: RawSound,
    notify: RawNotify,
    cooldowns: RawCooldowns,
    context: RawContext,
    hook: RawHook,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawSound {
    enabled: Option<bool>,
    volume: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawNotify {
    default_icon: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawCooldowns {
    enabled: Option<bool>,
    history: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawContext {
    morning: Option<String>,
    late: Option<String>,
    boost: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawHook {
    min_gap: Option<String>,
    ignore_commands: Option<Vec<String>>,
}

/// `~/` at the start of a path means the home folder.
fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

fn window(key: &str, value: Option<String>, default: (u32, u32)) -> Result<(u32, u32), String> {
    let Some(value) = value else {
        return Ok(default);
    };
    let (start, end) = parse_window(&value).map_err(|e| format!("context.{key}: {e}"))?;
    if start == end % (24 * 60) && !(start == 0 && end == 24 * 60) {
        return Err(format!(
            "context.{key}: start and end must differ (use 00:00-24:00 for all day)"
        ));
    }
    Ok((start, end))
}

/// Parses and validates the file's contents.
pub fn parse(text: &str) -> Result<Config, String> {
    let raw: Raw = toml::from_str(text).map_err(|e| e.to_string().trim_end().to_owned())?;
    let d = Config::default();

    let volume = raw.sound.volume.unwrap_or(d.volume);
    if !(0.0..=1.0).contains(&volume) {
        return Err(format!(
            "sound.volume: {volume} must be between 0.0 and 1.0"
        ));
    }
    let history = raw.cooldowns.history.unwrap_or(50);
    if !(1..=1000).contains(&history) {
        return Err(format!(
            "cooldowns.history: {history} must be between 1 and 1000"
        ));
    }
    let boost = raw.context.boost.unwrap_or(d.context.boost);
    if !(boost.is_finite() && boost >= 1.0) {
        return Err(format!("context.boost: {boost} must be at least 1.0"));
    }
    let min_gap = match raw.hook.min_gap {
        Some(gap) => parse_duration(&gap).map_err(|e| format!("hook.min_gap: {e}"))?,
        None => d.min_gap,
    };

    Ok(Config {
        packs_dir: raw.packs_dir.as_deref().map(expand_home),
        disabled_packs: raw.disabled_packs.unwrap_or_default(),
        sound_enabled: raw.sound.enabled.unwrap_or(d.sound_enabled),
        volume,
        default_icon: raw.notify.default_icon.unwrap_or(d.default_icon),
        cooldowns: raw.cooldowns.enabled.unwrap_or(d.cooldowns),
        history: usize::try_from(history).unwrap_or(d.history),
        context: ContextRules {
            morning: window("morning", raw.context.morning, d.context.morning)?,
            late: window("late", raw.context.late, d.context.late)?,
            boost,
        },
        min_gap,
        ignore_commands: raw.hook.ignore_commands.unwrap_or_default(),
    })
}

/// Where the config file is: `--config`, else `SHIVER_CONFIG`, else the
/// default. The flag says whether it was chosen explicitly.
pub fn locate(flag: Option<PathBuf>) -> anyhow::Result<(PathBuf, bool)> {
    if let Some(path) = flag {
        return Ok((path, true));
    }
    if let Some(path) = std::env::var_os("SHIVER_CONFIG").filter(|v| !v.is_empty()) {
        return Ok((PathBuf::from(path), true));
    }
    let dir = dirs::config_dir().context("can't find the config folder")?;
    Ok((dir.join("shiver").join("config.toml"), false))
}

/// Loads the config. A missing default file gives the defaults; anything
/// wrong with the file is an error naming it. Returns whether it exists.
pub fn load(path: &Path, explicit: bool) -> anyhow::Result<(Config, bool)> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let config = parse(&text)
                .map_err(|e| anyhow::anyhow!("invalid config {}: {e}", path.display()))?;
            Ok((config, true))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !explicit => {
            Ok((Config::default(), false))
        }
        Err(e) => bail!("can't read config {}: {e}", path.display()),
    }
}

fn time(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn window_str((start, end): (u32, u32)) -> String {
    format!("{}-{}", time(start), time(end))
}

fn toml_string(s: &str) -> String {
    toml::Value::String(s.to_owned()).to_string()
}

fn toml_list(items: &[String]) -> String {
    let items: Vec<_> = items.iter().map(|s| toml_string(s)).collect();
    format!("[{}]", items.join(", "))
}

/// Number with at least one decimal, as TOML writes floats.
fn float(x: f64) -> String {
    let s = x.to_string();
    if s.contains(['.', 'e', 'E']) {
        s
    } else {
        format!("{s}.0")
    }
}

impl Config {
    /// The settings as TOML, in the file's format. `packs_dir` is the one in
    /// effect (after flags and environment).
    pub fn to_toml(&self, packs_dir: &Path) -> String {
        format!(
            "packs_dir = {}\n\
             disabled_packs = {}\n\
             \n\
             [sound]\n\
             enabled = {}\n\
             volume = {}\n\
             \n\
             [notify]\n\
             default_icon = {}\n\
             \n\
             [cooldowns]\n\
             enabled = {}\n\
             history = {}\n\
             \n\
             [context]\n\
             morning = \"{}\"\n\
             late = \"{}\"\n\
             boost = {}\n\
             \n\
             [hook]\n\
             min_gap = \"{}\"\n\
             ignore_commands = {}\n",
            toml_string(&packs_dir.to_string_lossy()),
            toml_list(&self.disabled_packs),
            self.sound_enabled,
            float(self.volume),
            self.default_icon,
            self.cooldowns,
            self.history,
            window_str(self.context.morning),
            window_str(self.context.late),
            float(self.context.boost),
            format_duration(self.min_gap),
            toml_list(&self.ignore_commands),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(parse(""), Ok(Config::default()));
        let d = Config::default();
        assert_eq!(d.volume, 1.0);
        assert_eq!(d.history, 50);
        assert_eq!(d.context.morning, (300, 660));
        assert_eq!(d.context.late, (1320, 300));
        assert_eq!(d.context.boost, 3.0);
        assert_eq!(d.min_gap, 10);
    }

    #[test]
    fn parses_everything() {
        let c = parse(
            r#"
            packs_dir = "/srv/packs"
            disabled_packs = ["work"]
            [sound]
            enabled = false
            volume = 0.6
            [notify]
            default_icon = false
            [cooldowns]
            enabled = false
            history = 7
            [context]
            morning = "06:00-10:30"
            late = "23:00-04:00"
            boost = 2.5
            [hook]
            min_gap = "1h30m"
            ignore_commands = ["make"]
            "#,
        )
        .unwrap();
        assert_eq!(
            c,
            Config {
                packs_dir: Some("/srv/packs".into()),
                disabled_packs: vec!["work".into()],
                sound_enabled: false,
                volume: 0.6,
                default_icon: false,
                cooldowns: false,
                history: 7,
                context: ContextRules {
                    morning: (360, 630),
                    late: (1380, 240),
                    boost: 2.5,
                },
                min_gap: 90,
                ignore_commands: vec!["make".into()],
            }
        );
    }

    #[test]
    fn expands_home() {
        let c = parse("packs_dir = \"~/my-packs\"").unwrap();
        assert_eq!(
            c.packs_dir,
            Some(dirs::home_dir().unwrap().join("my-packs"))
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        let e = parse("[sound]\nvolum = 0.5\n").unwrap_err();
        assert!(e.contains("volum"), "{e}");
        let e = parse("colour = \"red\"\n").unwrap_err();
        assert!(e.contains("colour"), "{e}");
    }

    #[test]
    fn rejects_bad_values() {
        for (text, needle) in [
            ("[sound]\nvolume = 1.5", "sound.volume"),
            ("[cooldowns]\nhistory = 0", "cooldowns.history"),
            ("[context]\nboost = 0.5", "context.boost"),
            ("[context]\nlate = \"22-05\"", "context.late"),
            ("[context]\nmorning = \"09:00-09:00\"", "context.morning"),
            ("[hook]\nmin_gap = \"soon\"", "hook.min_gap"),
            ("[sound]\nenabled = \"yes\"", "enabled"),
            ("not toml at all", ""),
        ] {
            let e = parse(text).unwrap_err();
            assert!(e.contains(needle), "{text}: {e}");
        }
    }

    #[test]
    fn round_trips_through_toml() {
        let c = parse("disabled_packs = [\"a b\", \"c\"]\n[sound]\nvolume = 0.25\n").unwrap();
        let text = c.to_toml(Path::new("/p/packs"));
        let back = parse(&text).unwrap();
        assert_eq!(
            back,
            Config {
                packs_dir: Some("/p/packs".into()),
                ..c
            }
        );
    }
}
