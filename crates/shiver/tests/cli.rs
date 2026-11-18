//! End-to-end tests of the `shiver` command (everything except notifications).

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// A temporary packs folder, with its own history (state) folder next to it.
struct Packs {
    root: TempDir,
    packs: std::path::PathBuf,
}

impl Packs {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let packs = root.path().join("packs");
        std::fs::create_dir(&packs).unwrap();
        Packs { root, packs }
    }

    fn dir(&self) -> &Path {
        &self.packs
    }

    fn state(&self) -> std::path::PathBuf {
        self.root.path().join("state")
    }

    fn add(&self, folder: &str, pack_toml: &str) -> &Self {
        let folder = self.dir().join(folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("pack.toml"), pack_toml).unwrap();
        self
    }

    /// Runs `shiver --packs-dir <dir> <args>`.
    fn run(&self, args: &[&str]) -> Output {
        let mut cmd = shiver();
        cmd.env("XDG_STATE_HOME", self.state())
            .arg("--packs-dir")
            .arg(self.dir())
            .args(args);
        cmd.output().unwrap()
    }
}

/// A state folder shared by tests that don't care about history, so the
/// real `~/.local/state` is never touched. Removed when the tests exit... or
/// left in the temp folder if they crash, which is harmless.
fn shared_state() -> &'static Path {
    static STATE: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    STATE.get_or_init(|| tempfile::tempdir().unwrap()).path()
}

/// The binary, without any environment that would change its behaviour.
fn shiver() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_shiver"));
    cmd.env_remove("SHIVER_PACKS_DIR")
        .env_remove("NO_COLOR")
        .env("XDG_STATE_HOME", shared_state());
    cmd
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

const ONE_LINE: &str = "[voices.drift]\ndisplay = \"DRIFT\"\n\
                        [[lines]]\nvoice = \"drift\"\ntext = \"Water the plant.\"\n";

/// `drift` and `ember`, with lines tagged `late` and `idle`.
const SEVERAL: &str = "[voices.drift]\ndisplay = \"DRIFT\"\n[voices.ember]\ndisplay = \"EMBER\"\n\
    [[lines]]\nvoice = \"drift\"\ntext = \"drift late\"\ntags = [\"late\"]\n\
    [[lines]]\nvoice = \"drift\"\ntext = \"drift idle\"\ntags = [\"idle\"]\n\
    [[lines]]\nvoice = \"ember\"\ntext = \"ember late\"\ntags = [\"late\"]\n\
    [[lines]]\nvoice = \"ember\"\ntext = \"ember idle\"\ntags = [\"idle\"]\n";

#[test]
fn uses_packs_dir_flag() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let out = packs.run(&["say"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Water the plant."));
}

#[test]
fn uses_packs_dir_env() {
    let from_env = Packs::new();
    from_env.add(
        "p",
        "[voices.v]\n[[lines]]\nvoice = \"v\"\ntext = \"from env\"\n",
    );
    let from_flag = Packs::new();
    from_flag.add(
        "p",
        "[voices.v]\n[[lines]]\nvoice = \"v\"\ntext = \"from flag\"\n",
    );

    let out = shiver()
        .env("SHIVER_PACKS_DIR", from_env.dir())
        .arg("say")
        .output()
        .unwrap();
    assert!(stdout(&out).contains("from env"), "{}", stderr(&out));

    let out = shiver()
        .env("SHIVER_PACKS_DIR", from_env.dir())
        .arg("--packs-dir")
        .arg(from_flag.dir())
        .arg("say")
        .output()
        .unwrap();
    assert!(stdout(&out).contains("from flag"), "{}", stderr(&out));
}

#[test]
fn ignores_empty_packs_dir_env() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let out = shiver()
        .env("SHIVER_PACKS_DIR", "")
        .env("XDG_CONFIG_HOME", packs.dir().join("no-config"))
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("no-config/shiver/packs"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn prints_load_warnings_to_stderr() {
    let packs = Packs::new();
    packs.add(
        "p",
        &format!("{ONE_LINE}[[lines]]\nvoice = \"ghost\"\ntext = \"Boo.\"\n"),
    );
    let out = packs.run(&["say"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Water the plant."));
    assert!(!stdout(&out).contains("warning"));
    let err = stderr(&out);
    assert!(err.starts_with("shiver: warning: "), "{err}");
    assert!(err.contains("unknown voice `ghost`"), "{err}");
}

#[test]
fn say_prints_a_line() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let out = packs.run(&["say"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out),
        " DRIFT\n\
         ╭──────────────────╮\n\
         │ Water the plant. │\n\
         ╰──────────────────╯\n"
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn say_filters_by_voice_and_tag() {
    let packs = Packs::new();
    packs.add("p", SEVERAL);
    for seed in 0..20 {
        let seed = seed.to_string();
        let out = packs.run(&["say", "--voice", "ember", "--tag", "idle", "--seed", &seed]);
        assert!(stdout(&out).contains("ember idle"), "{}", stdout(&out));

        let out = packs.run(&["say", "--tag", "late", "--seed", &seed]);
        assert!(stdout(&out).contains(" late"), "{}", stdout(&out));
    }
}

#[test]
fn say_same_seed_same_line() {
    let packs = Packs::new();
    packs.add("p", SEVERAL);
    // Same seed and same (empty) history give the same line.
    let a = packs.run(&["--seed", "9", "say"]);
    std::fs::remove_dir_all(packs.state()).unwrap();
    let b = packs.run(&["say", "--seed", "9"]);
    assert_eq!(stdout(&a), stdout(&b));
}

#[test]
fn say_without_packs_fails() {
    let packs = Packs::new();
    let out = packs.run(&["say"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert_eq!(
        stderr(&out),
        format!(
            "shiver: no packs found in {}; run `shiver init` to install the default pack\n",
            packs.dir().display()
        )
    );
}

#[test]
fn say_without_match_fails() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let out = packs.run(&["say", "--tag", "nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stderr(&out), "shiver: no matching lines\n");
}

#[test]
fn text_requires_voice() {
    let packs = Packs::new();
    let out = packs.run(&["notify", "--text", "hi"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn text_conflicts_with_tag() {
    let packs = Packs::new();
    let out = packs.run(&["notify", "--voice", "v", "--text", "hi", "--tag", "late"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn list_shows_packs() {
    let packs = Packs::new();
    packs
        .add("a", SEVERAL)
        .add("b", "[[lines]]\nvoice = \"drift\"\ntext = \"borrowed\"\n");
    let out = packs.run(&["list"]);
    assert!(out.status.success());
    let a = packs.dir().join("a");
    let b = packs.dir().join("b");
    assert_eq!(
        stdout(&out),
        format!(
            "a (2 voices, 4 lines)\n  {}\n  voices: drift, ember\n\
             \n\
             b (0 voices, 1 line)\n  {}\n  voices: none\n",
            a.display(),
            b.display()
        )
    );
}

#[test]
fn list_without_packs() {
    let packs = Packs::new();
    let out = packs.run(&["list"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out),
        format!(
            "no packs found in {}\nrun `shiver init` to install the default pack\n",
            packs.dir().display()
        )
    );
}

const INIT_HINT: &str = "try `shiver say` or `shiver notify`; edit the pack to make it yours\n";

#[test]
fn init_creates_default_pack() {
    let packs = Packs::new();
    let file = packs.dir().join("default/pack.toml");
    let out = packs.run(&["init"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("created {}\n{INIT_HINT}", file.display())
    );
    assert!(file.is_file());

    let out = packs.run(&["list"]);
    assert!(
        stdout(&out).starts_with("default (5 voices, "),
        "{}",
        stdout(&out)
    );
}

#[test]
fn init_refuses_to_overwrite() {
    let packs = Packs::new();
    packs.add("default", "# my edits\n");
    let file = packs.dir().join("default/pack.toml");

    let out = packs.run(&["init"]);

    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out),
        format!(
            "shiver: {} already exists; use --force to overwrite\n",
            file.display()
        )
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "# my edits\n");
}

#[test]
fn init_force_overwrites() {
    let packs = Packs::new();
    packs.add("default", "# my edits\n");
    let file = packs.dir().join("default/pack.toml");

    let out = packs.run(&["init", "--force"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("replaced {}\n{INIT_HINT}", file.display())
    );
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("[voices.backbone]")
    );
}

#[test]
fn init_ignores_broken_packs() {
    let packs = Packs::new();
    packs.add("broken", "not = = toml");
    let out = packs.run(&["init"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stderr(&out), "");
}

/// A sandbox for `shiver timer`: a fake `systemctl` on `PATH` that logs its
/// arguments, and a temporary config folder for the unit files.
struct TimerEnv {
    root: TempDir,
}

const FAKE_SYSTEMCTL: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_SYSTEMCTL_LOG"
case "$*" in
  *is-active*)
    if [ -n "$FAKE_ACTIVE" ]; then echo active; exit 0; fi
    echo inactive; exit 3 ;;
  *NextElapseUSecRealtime*) echo "Wed 2026-11-18 10:47:00 GMT" ;;
  *"enable --now"*)
    if [ -n "$FAKE_FAIL" ]; then echo "Failed to enable unit: boom" >&2; exit 1; fi ;;
  *"disable --now"*)
    if [ -n "$FAKE_NOT_LOADED" ]; then echo "Unit shiver.timer not loaded." >&2; exit 5; fi ;;
esac
exit 0
"#;

impl TimerEnv {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let script = bin.join("systemctl");
        std::fs::write(&script, FAKE_SYSTEMCTL).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        TimerEnv { root }
    }

    fn units(&self) -> std::path::PathBuf {
        self.root.path().join("config/systemd/user")
    }

    fn unit(&self, name: &str) -> String {
        std::fs::read_to_string(self.units().join(name)).unwrap()
    }

    /// Commands the fake systemctl received, one per line.
    fn log(&self) -> String {
        std::fs::read_to_string(self.root.path().join("systemctl.log")).unwrap_or_default()
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let path = format!(
            "{}:{}",
            self.root.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = shiver();
        cmd.env("PATH", path)
            .env("XDG_CONFIG_HOME", self.root.path().join("config"))
            .env("FAKE_SYSTEMCTL_LOG", self.root.path().join("systemctl.log"))
            .env_remove("FAKE_ACTIVE")
            .env_remove("FAKE_FAIL")
            .env_remove("FAKE_NOT_LOADED")
            .args(args);
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd(args).output().unwrap()
    }

    /// Writes a unit file that shiver didn't write.
    fn add_foreign(&self, name: &str) {
        std::fs::create_dir_all(self.units()).unwrap();
        std::fs::write(self.units().join(name), "[Timer]\nOnCalendar=daily\n").unwrap();
    }
}

fn exe() -> String {
    std::fs::canonicalize(env!("CARGO_BIN_EXE_shiver"))
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn timer_enable_writes_units_and_starts_timer() {
    let env = TimerEnv::new();
    let out = env.run(&["timer", "enable"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!(
            "wrote {}\nwrote {}\n\
             schedule: every 45m between 09:00-21:00, jitter 20m (popups 09:00-20:35)\n\
             timer enabled; next popup: Wed 2026-11-18 10:47:00 GMT\n",
            env.units().join("shiver.service").display(),
            env.units().join("shiver.timer").display(),
        )
    );
    assert_eq!(stderr(&out), "");
    assert!(
        env.unit("shiver.service")
            .contains(&format!("ExecStart={} notify\n", exe()))
    );
    assert!(
        env.unit("shiver.timer")
            .contains("RandomizedDelaySec=20m\n")
    );
    assert_eq!(
        env.log(),
        "--user daemon-reload\n\
         --user enable --now shiver.timer\n\
         --user show shiver.timer --property=NextElapseUSecRealtime --value\n"
    );
}

#[test]
fn timer_enable_writes_packs_dir() {
    let env = TimerEnv::new();
    let packs = Packs::new();
    let out = env
        .cmd(&["timer", "enable"])
        .env("SHIVER_PACKS_DIR", packs.dir())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(env.unit("shiver.service").contains(&format!(
        "ExecStart={} --packs-dir {} notify\n",
        exe(),
        packs.dir().display()
    )));
}

#[test]
fn timer_enable_refuses_foreign_units() {
    let env = TimerEnv::new();
    env.add_foreign("shiver.timer");
    let out = env.run(&["timer", "enable"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out),
        format!(
            "shiver: {} wasn't written by shiver; use --force to replace it\n",
            env.units().join("shiver.timer").display()
        )
    );
    assert_eq!(env.unit("shiver.timer"), "[Timer]\nOnCalendar=daily\n");
    assert!(!env.units().join("shiver.service").exists());
    assert_eq!(env.log(), "");
}

#[test]
fn timer_enable_force_replaces_foreign_units() {
    let env = TimerEnv::new();
    env.add_foreign("shiver.timer");
    let out = env.run(&["timer", "enable", "--force"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        env.unit("shiver.timer")
            .starts_with("# Written by `shiver timer enable`")
    );
}

#[test]
fn timer_enable_replaces_its_own_units() {
    let env = TimerEnv::new();
    assert!(env.run(&["timer", "enable"]).status.success());
    let out = env.run(&["timer", "enable", "--every", "1h"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(env.unit("shiver.timer").contains("every 1h between"));
}

#[test]
fn timer_enable_reports_systemctl_failure() {
    let env = TimerEnv::new();
    let out = env
        .cmd(&["timer", "enable"])
        .env("FAKE_FAIL", "1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("`systemctl --user enable --now shiver.timer` failed"),
        "{err}"
    );
    assert!(err.contains("boom"), "{err}");
    assert!(
        env.units().join("shiver.timer").exists(),
        "files stay for a retry"
    );
}

#[test]
fn timer_enable_with_schedule_options() {
    let env = TimerEnv::new();
    let out = env.run(&[
        "timer",
        "enable",
        "--every",
        "1h",
        "--between",
        "08:00-18:00",
        "--jitter",
        "0m",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out)
            .contains("schedule: every 1h between 08:00-18:00, jitter 0m (popups 08:00-17:00)\n")
    );
    let timer = env.unit("shiver.timer");
    assert!(timer.contains("OnCalendar=*-*-* 08,09,10,11,12,13,14,15,16,17:00:00\n"));
    assert_eq!(timer.matches("OnCalendar=").count(), 1);

    for bad in [
        &["--every", "4m"][..],
        &["--jitter", "45m"],
        &["--between", "09:00-09:00"],
        &["--every", "soon"],
        &["--between", "9-5"],
    ] {
        let mut args = vec!["timer", "enable"];
        args.extend_from_slice(bad);
        let out = env.run(&args);
        assert_eq!(out.status.code(), Some(2), "{bad:?}: {}", stderr(&out));
    }
}

#[test]
fn timer_enable_warns_about_late_popups() {
    let env = TimerEnv::new();
    let out = env.run(&["timer", "enable", "--every", "50m", "--jitter", "30m"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stderr(&out),
        "shiver: warning: popups can arrive until 21:10, after the window ends at 21:00 \
         (last slot 20:40 + jitter 30m); use a smaller --jitter or an earlier end\n"
    );
    assert!(env.units().join("shiver.timer").exists());
}

#[test]
fn timer_disable_removes_units() {
    let env = TimerEnv::new();
    assert!(env.run(&["timer", "enable"]).status.success());
    let out = env.run(&["timer", "disable"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "timer disabled\n");
    assert!(!env.units().join("shiver.timer").exists());
    assert!(!env.units().join("shiver.service").exists());
    assert!(
        env.log()
            .ends_with("--user disable --now shiver.timer\n--user daemon-reload\n"),
        "{}",
        env.log()
    );
}

#[test]
fn timer_disable_when_not_installed() {
    let env = TimerEnv::new();
    let out = env
        .cmd(&["timer", "disable"])
        .env("FAKE_NOT_LOADED", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "timer disabled\n");
    assert_eq!(env.log(), "--user disable --now shiver.timer\n");
}

#[test]
fn timer_disable_keeps_foreign_units() {
    let env = TimerEnv::new();
    env.add_foreign("shiver.timer");
    let out = env.run(&["timer", "disable"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("wasn't written by shiver; left in place"));
    assert!(env.units().join("shiver.timer").exists());
}

#[test]
fn timer_status_enabled() {
    let env = TimerEnv::new();
    assert!(
        env.run(&["timer", "enable", "--between", "22:00-02:00"])
            .status
            .success()
    );
    let out = env
        .cmd(&["timer", "status"])
        .env("FAKE_ACTIVE", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "schedule: every 45m between 22:00-02:00, jitter 20m (popups 22:00-02:05)\n\
         timer enabled; next popup: Wed 2026-11-18 10:47:00 GMT\n"
    );
}

#[test]
fn timer_status_custom_schedule() {
    let env = TimerEnv::new();
    env.add_foreign("shiver.timer");
    let out = env
        .cmd(&["timer", "status"])
        .env("FAKE_ACTIVE", "1")
        .output()
        .unwrap();
    assert!(
        stdout(&out).starts_with("schedule: custom\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn timer_status_not_enabled() {
    let env = TimerEnv::new();
    let out = env.run(&["timer", "status"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "timer not enabled\n");
}

#[test]
fn say_does_not_repeat_lines() {
    let packs = Packs::new();
    packs.add(
        "p",
        "[voices.v]\n\
         [[lines]]\nvoice = \"v\"\ntext = \"one\"\n\
         [[lines]]\nvoice = \"v\"\ntext = \"two\"\n\
         [[lines]]\nvoice = \"v\"\ntext = \"three\"\n",
    );
    for seed in 0..5 {
        let _ = std::fs::remove_dir_all(packs.state());
        let mut seen: Vec<String> = (0..3)
            .map(|n| {
                let seed = (seed * 10 + n).to_string();
                let out = packs.run(&["say", "--seed", &seed]);
                assert!(out.status.success(), "{}", stderr(&out));
                stdout(&out)
            })
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 3, "seed {seed}: {seen:?}");
    }
}

#[test]
fn history_uses_state_dir() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let out = packs.run(&["say"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let history = std::fs::read_to_string(packs.state().join("shiver/history")).unwrap();
    let fields: Vec<_> = history.trim().split(' ').collect();
    assert_eq!(fields.len(), 3, "{history}");
    assert_eq!(fields[1].len(), 16);
    assert_eq!(fields[2], "drift");
}

#[test]
fn corrupt_history_only_warns() {
    let packs = Packs::new();
    packs.add("p", ONE_LINE);
    let file = packs.state().join("shiver/history");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "not a history line\n").unwrap();
    let out = packs.run(&["say"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Water the plant."));
    assert!(
        stderr(&out).contains("ignored 1 malformed line(s) in history"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn say_at_uses_context() {
    let packs = Packs::new();
    packs.add(
        "p",
        "[voices.v]\n\
         [[lines]]\nvoice = \"v\"\ntext = \"night line\"\ntags = [\"late\"]\n\
         [[lines]]\nvoice = \"v\"\ntext = \"dawn line\"\ntags = [\"morning\"]\n",
    );
    for seed in 0..10 {
        let seed = seed.to_string();
        let night = packs.run(&["say", "--at", "23:30", "--seed", &seed]);
        assert!(stdout(&night).contains("night line"), "{}", stderr(&night));
        let dawn = packs.run(&["say", "--at", "07:00", "--seed", &seed]);
        assert!(stdout(&dawn).contains("dawn line"), "{}", stderr(&dawn));
    }
    let noon = packs.run(&["say", "--at", "12:00"]);
    assert_eq!(noon.status.code(), Some(1));
    assert_eq!(stderr(&noon), "shiver: no matching lines\n");
}

#[test]
fn say_at_rejects_bad_time() {
    let packs = Packs::new();
    for bad in ["24:00", "7:00", "12:60", "noon"] {
        let out = packs.run(&["say", "--at", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad}");
    }
}
