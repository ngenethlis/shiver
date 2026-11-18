---
id: 0002
name: cli
status: done
created: 2026-11-18
---

# 0002: The shiver command

Add the `shiver` binary crate with `say`, `notify` and `list`, built on
`shiver-core` (spec 0001). Human-facing overview: [onepager.html](onepager.html).

## Commands

```
shiver [--packs-dir DIR] say    [--voice V] [--tag T]
shiver [--packs-dir DIR] notify [--voice V] [--tag T] [--no-sound]
shiver [--packs-dir DIR] notify --voice V --text TEXT [--sound FILE] [--no-sound]
shiver [--packs-dir DIR] list
```

Exit codes: 0 on success, 1 on an error, 2 on a usage error (clap's default).
Error messages go to stderr as `shiver: <message>`.

## Requirements

### R1: Binary crate

`crates/shiver` builds a binary named `shiver`. Dependencies: `shiver-core`,
`clap` (derive), `anyhow`, `dirs`, `notify-rust` (pure-Rust D-Bus backend,
no C libraries), `rand`, `unicode-width`. Integration tests use
`env!("CARGO_BIN_EXE_shiver")` and `tempfile`, no other test crates.

**Acceptance:**
- [x] `cargo build --workspace` succeeds on a machine without ALSA or D-Bus
      development headers.
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test` pass.

### R2: Packs folder and warnings

The packs folder is, in order of priority: `--packs-dir DIR`, the
`SHIVER_PACKS_DIR` environment variable, then `<config dir>/shiver/packs`
(`~/.config/shiver/packs` on Linux).

Every command loads the library first. Each load warning is printed to stderr
as `shiver: warning: <warning>`. If the folder can't be read, the command fails
with exit 1.

**Acceptance:**
- [x] Test `cli::uses_packs_dir_flag` passes.
- [x] Test `cli::uses_packs_dir_env` passes, and the flag beats the variable.
- [x] Test `cli::prints_load_warnings_to_stderr` passes: stdout has the line,
      stderr has the warning.

### R3: `shiver say`

Picks a line with the `--voice` and `--tag` filter and prints it as a speech
box (R4) to stdout.

- No packs loaded: `shiver: no packs found in <dir>`, exit 1.
- Packs loaded but no line matches: `shiver: no matching lines`, exit 1.

A hidden `--seed N` global option seeds the random pick, for tests.

**Acceptance:**
- [x] Test `cli::say_prints_a_line` passes.
- [x] Test `cli::say_filters_by_voice_and_tag` passes.
- [x] Test `cli::say_same_seed_same_line` passes.
- [x] Test `cli::say_without_packs_fails` passes.
- [x] Test `cli::say_without_match_fails` passes.

### R4: Speech box

```
 RESOLVE
╭──────────────────────────────────────╮
│ You've been staring at this bug for  │
│ an hour. Stand up.                   │
╰──────────────────────────────────────╯
```

- First line: a space, then the voice's display name.
- Text is word-wrapped to at most 60 display columns (measured with
  `unicode-width`). A word longer than 60 columns is split. Line breaks in the
  text are kept.
- The box is 4 columns wider than the longest wrapped line: `│ `, the line
  padded with spaces, ` │`.
- Colour: when stdout is a terminal and `NO_COLOR` is unset or empty, the
  display name is bold, and in the voice's colour if it has one (24-bit ANSI).
  Otherwise no escape codes at all.

**Acceptance:**
- [x] Test `stdout::tests::renders_box` passes (exact output for a short line).
- [x] Test `stdout::tests::wraps_at_60_columns` passes.
- [x] Test `stdout::tests::splits_long_words` passes.
- [x] Test `stdout::tests::keeps_line_breaks` passes.
- [x] Test `stdout::tests::measures_wide_characters` passes.
- [x] Test `stdout::tests::colours_name_only_when_enabled` passes.

### R5: `shiver notify`

Picks a line like `say` (same errors and exit codes) and sends a desktop
notification over the freedesktop notification spec:

| Field | Value |
|---|---|
| app name | `shiver` |
| summary | the voice's display name |
| body | the line's text, plain |
| icon | the voice's icon path, if any |
| timeout | the server's default |

Then plays the interjection's sound (R7) unless `--no-sound` is given. If the
notification can't be sent (e.g. no D-Bus session), exit 1 with the error.

**Acceptance:**
- [x] Test `notify::tests::builds_fields` passes: summary, body and icon come
      from the interjection.
- [x] Manual: on COSMIC, `shiver notify` shows a notification with the voice
      name and line, and plays the voice's sound.
- [x] Manual: `shiver notify --no-sound` shows the notification silently.

### R6: One-off `notify --text`

`--text TEXT` requires `--voice V` and can't be combined with `--tag`.

- If V is a loaded voice, its display name, icon and sound are used.
- Otherwise V itself is the display name, with no icon or sound.
- `--sound FILE` sets the sound, overriding the voice's.
- `--no-sound` turns sound off in every case.

Works with no packs installed.

**Acceptance:**
- [x] Test `cli::text_requires_voice` passes (exit 2).
- [x] Test `cli::text_conflicts_with_tag` passes (exit 2).
- [x] Test `notify::tests::one_off_uses_known_voice` passes.
- [x] Test `notify::tests::one_off_unknown_voice_uses_name` passes.
- [x] Test `notify::tests::one_off_sound_overrides_voice` passes.
- [x] Manual: `shiver notify --voice Test --text "Hello"` shows a notification
      titled `Test`.

### R7: Sound

Plays the sound file without waiting: spawn `pw-play <file>`, and if `pw-play`
isn't installed, `paplay <file>`. The player's stdin, stdout and stderr are
discarded and shiver exits without waiting for it.

- Neither player installed: `shiver: warning: no sound player found (tried pw-play, paplay)`,
  exit 0.
- The file doesn't exist: `shiver: warning: sound file <path> not found`, exit 0,
  no player started.

**Acceptance:**
- [x] Test `sound::tests::uses_pw_play_first` passes.
- [x] Test `sound::tests::falls_back_to_paplay` passes.
- [x] Test `sound::tests::warns_when_no_player` passes.
- [x] Test `sound::tests::warns_when_file_missing` passes.

(The tests swap the process spawner for a fake, so no audio is played.)

### R8: `shiver list`

Prints each pack in folder order:

```
desk (3 voices, 12 lines)
  /home/me/.config/shiver/packs/desk
  voices: drift, ember, resolve
```

A blank line separates packs. A pack with no voices of its own prints
`voices: none`. With no packs: `no packs found in <dir>` on stdout, exit 0.

**Acceptance:**
- [x] Test `cli::list_shows_packs` passes (exact output).
- [x] Test `cli::list_without_packs` passes.

### R9: Output modules

Terminal, notification and sound outputs are separate modules (`stdout`,
`notify`, `sound`) implementing one `Output` trait that takes an
`Interjection`. `main` only parses arguments, loads the library and calls
outputs.

**Acceptance:**
- [x] Review: the three modules exist and implement `Output`; no output logic
      in `main.rs`.

### R10: Original test content

**Acceptance:**
- [x] Review: every test pack and line is original text written for these tests.

## Out of scope

- systemd timer and service files for random popups.
- An example pack in the repo.
- Shell hooks, context-aware selection, cooldowns, a config file, the overlay.
- Notification actions, urgency, custom timeouts or markup.
