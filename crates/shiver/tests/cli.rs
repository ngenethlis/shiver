//! End-to-end tests of the `shiver` command (everything except notifications).

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// A temporary packs folder.
struct Packs(TempDir);

impl Packs {
    fn new() -> Self {
        Packs(tempfile::tempdir().unwrap())
    }

    fn dir(&self) -> &Path {
        self.0.path()
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
        cmd.arg("--packs-dir").arg(self.dir()).args(args);
        cmd.output().unwrap()
    }
}

/// The binary, without any environment that would change its behaviour.
fn shiver() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_shiver"));
    cmd.env_remove("SHIVER_PACKS_DIR").env_remove("NO_COLOR");
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
    let a = packs.run(&["--seed", "9", "say"]);
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
        format!("shiver: no packs found in {}\n", packs.dir().display())
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
        format!("no packs found in {}\n", packs.dir().display())
    );
}
