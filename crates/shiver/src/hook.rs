//! The shell hook: `shiver hook zsh|bash` prints a snippet that runs
//! `shiver on-fail` after a failed command; `on-fail` decides whether a voice
//! speaks up.

use std::path::{Path, PathBuf};

use crate::timer::format_duration;

/// Exit codes that aren't real failures: Ctrl-C, broken pipe, Ctrl-Z.
const IGNORED_CODES: [i32; 4] = [0, 130, 141, 148];

/// Commands where a non-zero exit is a normal answer, not a failure.
const IGNORED_COMMANDS: [&str; 13] = [
    "grep", "egrep", "fgrep", "rg", "ag", "diff", "cmp", "test", "[", "[[", "false", "which",
    "type",
];

/// Words that run another command, skipped when finding the real one.
const WRAPPERS: [&str; 3] = ["sudo", "command", "time"];

/// The command a shell line runs: skips `VAR=value` assignments, wrappers
/// like `sudo`, and their `-flags`.
fn command_name(line: &str) -> Option<&str> {
    let mut after_wrapper = false;
    line.split_whitespace().find(|word| {
        let skip = (word.contains('=') && !word.starts_with('='))
            || WRAPPERS.contains(word)
            || (after_wrapper && word.starts_with('-'));
        after_wrapper = WRAPPERS.contains(word) || (after_wrapper && word.starts_with('-'));
        !skip
    })
}

/// Why `on-fail` didn't fire, if it doesn't.
/// `also_ignored` adds to the built-in ignored commands.
pub fn skip_reason(
    code: i32,
    command: Option<&str>,
    also_ignored: &[String],
    last_fail: Option<u64>,
    now: u64,
    min_gap: u32,
) -> Option<String> {
    if IGNORED_CODES.contains(&code) {
        return Some(format!("ignored exit code {code}"));
    }
    if let Some(name) = command.and_then(command_name)
        && (IGNORED_COMMANDS.contains(&name) || also_ignored.iter().any(|c| c == name))
    {
        return Some(format!("ignored command {name}"));
    }
    if let Some(last) = last_fail {
        let elapsed = now.saturating_sub(last);
        if elapsed < u64::from(min_gap) * 60 {
            return Some(format!(
                "last popup {} ago (min gap {})",
                format_duration(u32::try_from(elapsed / 60).unwrap_or(u32::MAX)),
                format_duration(min_gap)
            ));
        }
    }
    None
}

/// `<state dir>/shiver/last-fail`.
pub fn last_fail_path() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("shiver").join("last-fail"))
}

/// When the hook last fired, if known.
pub fn read_last_fail(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Records that the hook fired. Failure only means the rate limit may not hold.
pub fn write_last_fail(path: &Path, now: u64) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(path, format!("{now}\n")) {
        crate::output::warn(format!("can't write {}: {e}", path.display()));
    }
}

/// Quotes a word for a POSIX shell.
fn shell_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// Shells `shiver hook` supports.
#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
}

/// The snippet to `eval` in the shell's rc file.
pub fn snippet(shell: Shell, exe: &Path, min_gap: Option<u32>, no_sound: bool) -> String {
    let mut run = format!(
        "{} on-fail --exit-code \"$code\" --command \"$__shiver_cmd\"",
        shell_quote(&exe.to_string_lossy())
    );
    if let Some(gap) = min_gap {
        run.push_str(&format!(" --min-gap {}", format_duration(gap)));
    }
    if no_sound {
        run.push_str(" --no-sound");
    }
    let name = match shell {
        Shell::Zsh => "zsh",
        Shell::Bash => "bash",
    };
    let fire = format!("    ( {run} >/dev/null 2>&1 & )\n");
    let header = format!(
        "# shiver: a voice may speak up when a command fails.\n\
         # Added by: eval \"$(shiver hook {name})\"\n"
    );
    match shell {
        Shell::Zsh => format!(
            "{header}\
             __shiver_cmd=\n\
             autoload -Uz add-zsh-hook\n\
             __shiver_preexec() {{ __shiver_cmd=$1; }}\n\
             __shiver_precmd() {{\n\
             \x20 local code=$?\n\
             \x20 if [[ -n $__shiver_cmd && $code -ne 0 ]]; then\n\
             {fire}\
             \x20 fi\n\
             \x20 __shiver_cmd=\n\
             }}\n\
             add-zsh-hook preexec __shiver_preexec\n\
             add-zsh-hook precmd __shiver_precmd\n"
        ),
        // bash has no pre-command hook. At each prompt, the newest history
        // entry is the command that just ran, if its number changed.
        Shell::Bash => format!(
            "{header}\
             # Reads the shell history: commands that aren't saved to it (e.g. with\n\
             # HISTCONTROL=ignorespace or repeated with ignoredups) don't trigger it.\n\
             __shiver_last=\n\
             __shiver_precmd() {{\n\
             \x20 local code=$? entry num __shiver_cmd\n\
             \x20 entry=$(HISTTIMEFORMAT= builtin history 1)\n\
             \x20 entry=${{entry#\"${{entry%%[! ]*}}\"}}\n\
             \x20 num=${{entry%%[!0-9]*}}\n\
             \x20 __shiver_cmd=${{entry#\"$num\"}}\n\
             \x20 __shiver_cmd=${{__shiver_cmd#\\*}}\n\
             \x20 __shiver_cmd=${{__shiver_cmd#\"${{__shiver_cmd%%[! ]*}}\"}}\n\
             \x20 if [[ -n $num && -n $__shiver_last && $num != \"$__shiver_last\" && $code -ne 0 ]]; then\n\
             {fire}\
             \x20 fi\n\
             \x20 __shiver_last=${{num:-none}}\n\
             }}\n\
             PROMPT_COMMAND=\"__shiver_precmd${{PROMPT_COMMAND:+; $PROMPT_COMMAND}}\"\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn skip(code: i32, cmd: Option<&str>) -> Option<String> {
        skip_reason(code, cmd, &[], None, NOW, 10)
    }

    #[test]
    fn ignores_exit_codes() {
        for code in [0, 130, 141, 148] {
            assert_eq!(skip(code, None), Some(format!("ignored exit code {code}")));
        }
        for code in [1, 2, 101, 127, 137] {
            assert_eq!(skip(code, None), None, "code {code}");
        }
    }

    #[test]
    fn ignores_commands() {
        for (line, name) in [
            ("grep -r foo .", "grep"),
            ("sudo grep foo /etc", "grep"),
            ("sudo -E rg foo", "rg"),
            ("LC_ALL=C grep x", "grep"),
            ("time diff a b", "diff"),
            ("[ -f x ]", "["),
            ("command which cargo", "which"),
        ] {
            assert_eq!(
                skip(1, Some(line)),
                Some(format!("ignored command {name}")),
                "{line}"
            );
        }
        for line in [
            "cargo build",
            "make",
            "sudo apt install x",
            "FOO=1 cargo test",
            "",
        ] {
            assert_eq!(skip(1, Some(line)), None, "{line}");
        }
    }

    #[test]
    fn rate_limit() {
        let gap = 10;
        let just_before = NOW - 10 * 60 + 1;
        assert_eq!(
            skip_reason(1, Some("make"), &[], Some(just_before), NOW, gap),
            Some("last popup 9m ago (min gap 10m)".into())
        );
        assert_eq!(
            skip_reason(1, Some("make"), &[], Some(NOW - 10 * 60), NOW, gap),
            None
        );
        assert_eq!(
            skip_reason(1, Some("make"), &[], Some(NOW - 3600), NOW, gap),
            None
        );
    }

    #[test]
    fn reads_and_writes_last_fail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state/shiver/last-fail");
        assert_eq!(read_last_fail(&path), None);
        write_last_fail(&path, NOW);
        assert_eq!(read_last_fail(&path), Some(NOW));
    }

    #[test]
    fn quotes_for_the_shell() {
        assert_eq!(shell_quote("/usr/bin/shiver"), "'/usr/bin/shiver'");
        assert_eq!(shell_quote("/it's/here"), r"'/it'\''s/here'");
    }
}
