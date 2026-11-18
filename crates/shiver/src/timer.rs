//! `shiver timer`: random popups with a systemd user timer.
//!
//! A [`Schedule`] (every N minutes inside a daily window, plus a random delay)
//! is turned into calendar slots, because systemd can't limit an interval timer
//! to certain hours of the day.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::output;

/// First line of every unit file shiver writes; used to recognise them.
const HEADER: &str = "# Written by `shiver timer enable`. Remove with `shiver timer disable`.";
/// First line of the example files in `contrib/systemd/`.
#[cfg(test)]
const CONTRIB_HEADER: &str =
    "# Example unit for shiver; see the README. Or run `shiver timer enable`.";
const SCHEDULE_PREFIX: &str = "# Schedule: ";

pub const SERVICE: &str = "shiver.service";
pub const TIMER: &str = "shiver.timer";

const MINUTES_PER_DAY: u32 = 24 * 60;
const MIN_EVERY: u32 = 5;

/// When popups happen. All values are in minutes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Schedule {
    every: u32,
    /// Window start, minutes after midnight (0..1440).
    start: u32,
    /// Window end, minutes after midnight (1..=1440). Earlier than `start`
    /// means the window crosses midnight.
    end: u32,
    jitter: u32,
}

impl Schedule {
    /// Builds a schedule, checking the rules that involve more than one option.
    pub fn new(every: u32, (start, end): (u32, u32), jitter: u32) -> Result<Self, String> {
        if every < MIN_EVERY {
            return Err(format!(
                "--every must be at least {}",
                format_duration(MIN_EVERY)
            ));
        }
        if start == end % MINUTES_PER_DAY && !(start == 0 && end == MINUTES_PER_DAY) {
            return Err("--between start and end must differ (use 00:00-24:00 for all day)".into());
        }
        if jitter >= every {
            return Err(format!(
                "--jitter ({}) must be less than --every ({}) so popups stay in order",
                format_duration(jitter),
                format_duration(every)
            ));
        }
        Ok(Schedule {
            every,
            start,
            end,
            jitter,
        })
    }

    /// Length of the window in minutes.
    fn length(&self) -> u32 {
        if self.end > self.start {
            self.end - self.start
        } else {
            self.end + MINUTES_PER_DAY - self.start
        }
    }

    /// Minutes after the window start of each slot.
    fn offsets(&self) -> impl Iterator<Item = u32> {
        let (every, length) = (self.every, self.length());
        (0..)
            .map(move |k| k * every)
            .take_while(move |&o| o < length)
    }

    /// Slot times as minutes after midnight, in order.
    pub fn slots(&self) -> Vec<u32> {
        self.offsets()
            .map(|o| (self.start + o) % MINUTES_PER_DAY)
            .collect()
    }

    /// `OnCalendar=` values: one per distinct minute, ascending, listing hours.
    fn calendar(&self) -> Vec<String> {
        let mut by_minute: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
        for slot in self.slots() {
            by_minute.entry(slot % 60).or_default().push(slot / 60);
        }
        by_minute
            .into_iter()
            .map(|(minute, mut hours)| {
                hours.sort_unstable();
                let hours: Vec<_> = hours.iter().map(|h| format!("{h:02}")).collect();
                format!("*-*-* {}:{minute:02}:00", hours.join(","))
            })
            .collect()
    }

    /// Minutes after the window start of the last possible popup.
    fn last_popup_offset(&self) -> u32 {
        self.offsets().last().unwrap_or(0) + self.jitter
    }

    /// `every 45m between 09:00-21:00, jitter 20m`
    fn summary(&self) -> String {
        format!(
            "every {} between {}-{}, jitter {}",
            format_duration(self.every),
            format_time(self.start),
            format_end(self.end),
            format_duration(self.jitter)
        )
    }

    /// The summary plus when popups can actually arrive.
    pub fn describe(&self) -> String {
        format!(
            "{} (popups {}-{})",
            self.summary(),
            format_time(self.start),
            format_time(self.start + self.last_popup_offset())
        )
    }

    /// A warning if the jitter lets popups arrive after the window ends.
    pub fn late_warning(&self) -> Option<String> {
        let last = self.last_popup_offset();
        if last <= self.length() {
            return None;
        }
        let last_slot = self.offsets().last().unwrap_or(0);
        Some(format!(
            "popups can arrive until {}, after the window ends at {} (last slot {} + jitter {}); \
             use a smaller --jitter or an earlier end",
            format_time(self.start + last),
            format_end(self.end),
            format_time(self.start + last_slot),
            format_duration(self.jitter)
        ))
    }

    /// Reads a schedule back from the `# Schedule:` comment of a timer file.
    fn from_timer_file(text: &str) -> Option<Self> {
        let line = text.lines().find_map(|l| l.strip_prefix(SCHEDULE_PREFIX))?;
        let rest = line.strip_prefix("every ")?;
        let (every, rest) = rest.split_once(" between ")?;
        let (window, jitter) = rest.split_once(", jitter ")?;
        Schedule::new(
            parse_duration(every).ok()?,
            parse_window(window).ok()?,
            parse_duration(jitter).ok()?,
        )
        .ok()
    }
}

/// Parses `45m`, `2h` or `1h30m` into minutes.
pub fn parse_duration(s: &str) -> Result<u32, String> {
    let err = || format!("invalid duration `{s}` (use e.g. 45m, 2h or 1h30m)");
    let (hours, rest) = match s.split_once('h') {
        Some((h, rest)) => (h.parse::<u32>().map_err(|_| err())?, rest),
        None => (0, s),
    };
    let minutes = if rest.is_empty() {
        if hours == 0 && s.is_empty() {
            return Err(err());
        }
        0
    } else {
        rest.strip_suffix('m')
            .filter(|m| !m.is_empty())
            .and_then(|m| m.parse::<u32>().ok())
            .ok_or_else(err)?
    };
    hours
        .checked_mul(60)
        .and_then(|h| h.checked_add(minutes))
        .ok_or_else(err)
}

/// Parses `HH:MM-HH:MM` into (start, end) minutes after midnight.
pub fn parse_window(s: &str) -> Result<(u32, u32), String> {
    let err = || format!("invalid window `{s}` (use e.g. 09:00-21:00)");
    let (a, b) = s.split_once('-').ok_or_else(err)?;
    let time = |t: &str| -> Option<u32> {
        let (h, m) = t.split_once(':')?;
        if h.len() != 2 || m.len() != 2 {
            return None;
        }
        let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
        (m < 60 && (h < 24 || (h == 24 && m == 0))).then_some(h * 60 + m)
    };
    let start = time(a).filter(|&t| t < MINUTES_PER_DAY).ok_or_else(err)?;
    let end = time(b).ok_or_else(err)?;
    Ok((start, end))
}

/// `90` → `1h30m`, `60` → `1h`, `45` → `45m`, `0` → `0m`.
pub fn format_duration(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

/// Minutes (any number) as a time of day.
fn format_time(minutes: u32) -> String {
    let m = minutes % MINUTES_PER_DAY;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Like [`format_time`], but 1440 is `24:00`.
fn format_end(minutes: u32) -> String {
    if minutes == MINUTES_PER_DAY {
        "24:00".to_owned()
    } else {
        format_time(minutes)
    }
}

/// Quotes a value for an `ExecStart=` line.
fn quote(value: &str) -> String {
    let escaped = value.replace('%', "%%").replace('$', "$$");
    if escaped
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\'))
    {
        format!("\"{}\"", escaped.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        escaped
    }
}

/// The service unit. `exec` is inserted as is (already quoted).
fn render_service(header: &str, exec: &str, packs_dir: Option<&Path>) -> String {
    let args = match packs_dir {
        Some(dir) => format!("--packs-dir {} ", quote(&dir.to_string_lossy())),
        None => String::new(),
    };
    format!(
        "{header}\n\
         [Unit]\n\
         Description=shiver: a random inner-voice popup\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={exec} {args}notify\n"
    )
}

/// The timer unit.
fn render_timer(header: &str, schedule: &Schedule) -> String {
    let mut s = format!(
        "{header}\n\
         {SCHEDULE_PREFIX}{}\n\
         [Unit]\n\
         Description=shiver: a random popup every {} between {}-{}\n\
         \n\
         [Timer]\n",
        schedule.summary(),
        format_duration(schedule.every),
        format_time(schedule.start),
        format_end(schedule.end),
    );
    for calendar in schedule.calendar() {
        let _ = writeln!(s, "OnCalendar={calendar}");
    }
    let _ = write!(
        s,
        "RandomizedDelaySec={}\n\
         AccuracySec=1min\n\
         Persistent=false\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n",
        format_duration(schedule.jitter)
    );
    s
}

/// Runs `systemctl --user <args>` and returns its stdout.
pub type Systemctl<'a> = dyn FnMut(&[&str]) -> anyhow::Result<String> + 'a;

/// The real `systemctl --user`.
pub fn systemctl(args: &[&str]) -> anyhow::Result<String> {
    let out = std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .context("can't run systemctl")?;
    if !out.status.success() {
        bail!(
            "`systemctl --user {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Whether `path` exists and wasn't written by shiver.
fn is_foreign(path: &Path) -> anyhow::Result<bool> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(!text.starts_with(HEADER)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("can't read {}", path.display())),
    }
}

/// The `timer enabled; next popup: ...` line.
fn next_popup(systemctl: &mut Systemctl) -> anyhow::Result<String> {
    let next = systemctl(&[
        "show",
        TIMER,
        "--property=NextElapseUSecRealtime",
        "--value",
    ])?;
    let next = next.trim();
    let next = if next.is_empty() { "unknown" } else { next };
    Ok(format!("timer enabled; next popup: {next}"))
}

/// What `enable` needs.
pub struct Enable<'a> {
    pub units_dir: &'a Path,
    pub exe: &'a Path,
    pub packs_dir: Option<&'a Path>,
    pub schedule: Schedule,
    pub force: bool,
}

/// Writes both units and starts the timer. Returns the lines to print.
pub fn enable(opts: &Enable, systemctl: &mut Systemctl) -> anyhow::Result<Vec<String>> {
    let service_path = opts.units_dir.join(SERVICE);
    let timer_path = opts.units_dir.join(TIMER);
    if !opts.force {
        for path in [&service_path, &timer_path] {
            if is_foreign(path)? {
                bail!(
                    "{} wasn't written by shiver; use --force to replace it",
                    path.display()
                );
            }
        }
    }

    std::fs::create_dir_all(opts.units_dir)
        .with_context(|| format!("can't create {}", opts.units_dir.display()))?;
    let service = render_service(HEADER, &quote(&opts.exe.to_string_lossy()), opts.packs_dir);
    let timer = render_timer(HEADER, &opts.schedule);
    let mut lines = Vec::new();
    for (path, text) in [(&service_path, service), (&timer_path, timer)] {
        std::fs::write(path, text).with_context(|| format!("can't write {}", path.display()))?;
        lines.push(format!("wrote {}", path.display()));
    }
    lines.push(format!("schedule: {}", opts.schedule.describe()));
    if let Some(warning) = opts.schedule.late_warning() {
        output::warn(warning);
    }

    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", TIMER])?;
    lines.push(next_popup(systemctl)?);
    Ok(lines)
}

/// Stops the timer and removes the units shiver wrote. Returns lines to print.
pub fn disable(units_dir: &Path, systemctl: &mut Systemctl) -> anyhow::Result<Vec<String>> {
    // Fails when the unit isn't loaded, which is fine.
    let _ = systemctl(&["disable", "--now", TIMER]);

    let mut removed = false;
    for name in [TIMER, SERVICE] {
        let path = units_dir.join(name);
        if !path.exists() {
            continue;
        }
        if is_foreign(&path)? {
            output::warn(format!(
                "{} wasn't written by shiver; left in place",
                path.display()
            ));
            continue;
        }
        std::fs::remove_file(&path).with_context(|| format!("can't remove {}", path.display()))?;
        removed = true;
    }
    if removed {
        systemctl(&["daemon-reload"])?;
    }
    Ok(vec!["timer disabled".to_owned()])
}

/// Describes whether the timer is running. Returns lines to print.
pub fn status(units_dir: &Path, systemctl: &mut Systemctl) -> anyhow::Result<Vec<String>> {
    let active = systemctl(&["is-active", TIMER])
        .map(|out| out.trim() == "active")
        .unwrap_or(false);
    if !active {
        return Ok(vec!["timer not enabled".to_owned()]);
    }
    let schedule = std::fs::read_to_string(units_dir.join(TIMER))
        .ok()
        .and_then(|text| Schedule::from_timer_file(&text))
        .map_or_else(|| "custom".to_owned(), |s| s.describe());
    Ok(vec![
        format!("schedule: {schedule}"),
        next_popup(systemctl)?,
    ])
}

/// `<config dir>/systemd/user`.
pub fn units_dir() -> anyhow::Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("can't find the config folder")?
        .join("systemd")
        .join("user"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_schedule() -> Schedule {
        Schedule::new(45, (9 * 60, 21 * 60), 20).unwrap()
    }

    fn schedule(every: &str, window: &str, jitter: &str) -> Result<Schedule, String> {
        Schedule::new(
            parse_duration(every)?,
            parse_window(window)?,
            parse_duration(jitter)?,
        )
    }

    fn times(slots: &[u32]) -> Vec<String> {
        slots.iter().map(|&s| format_time(s)).collect()
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("45m"), Ok(45));
        assert_eq!(parse_duration("2h"), Ok(120));
        assert_eq!(parse_duration("1h30m"), Ok(90));
        assert_eq!(parse_duration("0m"), Ok(0));
        assert_eq!(parse_duration("90m"), Ok(90));
        for bad in ["", "45", "m", "h", "1h30", "-5m", "1.5h", "5s", "1m2h"] {
            assert!(parse_duration(bad).is_err(), "`{bad}` should be rejected");
        }
        assert_eq!(format_duration(90), "1h30m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(45), "45m");
        assert_eq!(format_duration(0), "0m");
    }

    #[test]
    fn parses_window() {
        assert_eq!(parse_window("09:00-21:00"), Ok((540, 1260)));
        assert_eq!(parse_window("00:00-24:00"), Ok((0, 1440)));
        assert_eq!(parse_window("22:00-02:00"), Ok((1320, 120)));
        for bad in [
            "9:00-21:00",
            "09:00",
            "09:00-21:60",
            "24:00-02:00",
            "09:00-24:30",
            "25:00-26:00",
            "ab:cd-ef:gh",
        ] {
            assert!(parse_window(bad).is_err(), "`{bad}` should be rejected");
        }
    }

    #[test]
    fn computes_slots() {
        let slots = times(&default_schedule().slots());
        assert_eq!(slots.len(), 16);
        assert_eq!(slots[..4], ["09:00", "09:45", "10:30", "11:15"]);
        assert_eq!(slots.last().unwrap(), "20:15");
    }

    #[test]
    fn computes_slots_across_midnight() {
        let s = schedule("45m", "22:00-02:00", "20m").unwrap();
        assert_eq!(
            times(&s.slots()),
            ["22:00", "22:45", "23:30", "00:15", "01:00", "01:45"]
        );
        let timer = render_timer(HEADER, &s);
        for line in [
            "OnCalendar=*-*-* 01,22:00:00",
            "OnCalendar=*-*-* 00:15:00",
            "OnCalendar=*-*-* 23:30:00",
            "OnCalendar=*-*-* 01,22:45:00",
        ] {
            assert!(timer.contains(line), "missing `{line}` in:\n{timer}");
        }
        assert_eq!(timer.matches("OnCalendar=").count(), 4);
    }

    #[test]
    fn rejects_invalid_schedules() {
        let too_often = schedule("4m", "09:00-21:00", "0m").unwrap_err();
        assert!(too_often.contains("at least 5m"), "{too_often}");

        let empty = schedule("45m", "09:00-09:00", "0m").unwrap_err();
        assert!(empty.contains("must differ"), "{empty}");

        let jitter = schedule("30m", "09:00-21:00", "30m").unwrap_err();
        assert!(jitter.contains("must be less than --every"), "{jitter}");

        assert!(
            schedule("45m", "00:00-24:00", "0m").is_ok(),
            "all day is fine"
        );
    }

    #[test]
    fn describes_schedule() {
        assert_eq!(
            default_schedule().describe(),
            "every 45m between 09:00-21:00, jitter 20m (popups 09:00-20:35)"
        );
        assert_eq!(
            schedule("45m", "22:00-02:00", "20m").unwrap().describe(),
            "every 45m between 22:00-02:00, jitter 20m (popups 22:00-02:05)"
        );
        assert_eq!(
            schedule("1h30m", "00:00-24:00", "0m").unwrap().describe(),
            "every 1h30m between 00:00-24:00, jitter 0m (popups 00:00-22:30)"
        );
    }

    #[test]
    fn warns_when_popups_run_late() {
        assert_eq!(
            schedule("50m", "09:00-21:00", "30m")
                .unwrap()
                .late_warning(),
            Some(
                "popups can arrive until 21:10, after the window ends at 21:00 \
                 (last slot 20:40 + jitter 30m); use a smaller --jitter or an earlier end"
                    .into()
            )
        );
        assert_eq!(default_schedule().late_warning(), None);
        // 20:40 + 20m lands exactly on 21:00: not late.
        assert_eq!(
            schedule("50m", "09:00-21:00", "20m")
                .unwrap()
                .late_warning(),
            None
        );
        // Across midnight: 01:45 + 20m = 02:05, after 02:00.
        let w = schedule("45m", "22:00-02:00", "20m")
            .unwrap()
            .late_warning();
        assert!(w.unwrap().contains("until 02:05"));
    }

    #[test]
    fn renders_service() {
        assert_eq!(
            render_service(HEADER, "/usr/bin/shiver", None),
            format!(
                "{HEADER}\n[Unit]\nDescription=shiver: a random inner-voice popup\n\n\
                 [Service]\nType=oneshot\nExecStart=/usr/bin/shiver notify\n"
            )
        );
    }

    #[test]
    fn renders_service_with_packs_dir() {
        let s = render_service(HEADER, "/usr/bin/shiver", Some(Path::new("/home/me/packs")));
        assert!(s.contains("ExecStart=/usr/bin/shiver --packs-dir /home/me/packs notify\n"));
    }

    #[test]
    fn quotes_paths_with_spaces() {
        assert_eq!(quote("/opt/my apps/shiver"), "\"/opt/my apps/shiver\"");
        assert_eq!(quote("/a/100%/b"), "/a/100%%/b");
        assert_eq!(quote("/a/$HOME"), "/a/$$HOME");
        assert_eq!(quote("/a \"q\"\\b"), "\"/a \\\"q\\\"\\\\b\"");
        let s = render_service(HEADER, &quote("/x y/shiver"), Some(Path::new("/p q")));
        assert!(s.contains("ExecStart=\"/x y/shiver\" --packs-dir \"/p q\" notify\n"));
    }

    #[test]
    fn renders_default_timer() {
        assert_eq!(
            render_timer(HEADER, &default_schedule()),
            format!(
                "{HEADER}\n\
                 # Schedule: every 45m between 09:00-21:00, jitter 20m\n\
                 [Unit]\n\
                 Description=shiver: a random popup every 45m between 09:00-21:00\n\
                 \n\
                 [Timer]\n\
                 OnCalendar=*-*-* 09,12,15,18:00:00\n\
                 OnCalendar=*-*-* 11,14,17,20:15:00\n\
                 OnCalendar=*-*-* 10,13,16,19:30:00\n\
                 OnCalendar=*-*-* 09,12,15,18:45:00\n\
                 RandomizedDelaySec=20m\n\
                 AccuracySec=1min\n\
                 Persistent=false\n\
                 \n\
                 [Install]\n\
                 WantedBy=timers.target\n"
            )
        );
    }

    #[test]
    fn renders_hourly_timer() {
        let t = render_timer(HEADER, &schedule("1h", "08:00-18:00", "0m").unwrap());
        assert_eq!(t.matches("OnCalendar=").count(), 1);
        assert!(t.contains("OnCalendar=*-*-* 08,09,10,11,12,13,14,15,16,17:00:00\n"));
        assert!(t.contains("RandomizedDelaySec=0m\n"));
    }

    #[test]
    fn reads_schedule_back_from_timer_file() {
        let s = schedule("1h30m", "22:00-02:00", "10m").unwrap();
        assert_eq!(
            Schedule::from_timer_file(&render_timer(HEADER, &s)),
            Some(s)
        );
        assert_eq!(
            Schedule::from_timer_file("[Timer]\nOnCalendar=daily\n"),
            None
        );
    }

    #[test]
    fn contrib_files_match_rendered() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contrib/systemd");
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
        assert_eq!(
            read(SERVICE),
            render_service(CONTRIB_HEADER, "%h/.cargo/bin/shiver", None)
        );
        assert_eq!(
            read(TIMER),
            render_timer(CONTRIB_HEADER, &default_schedule())
        );
    }
}
