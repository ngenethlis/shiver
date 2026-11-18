---
id: 0011
name: config
status: done
created: 2026-11-18
---

# 0011: Config file

An optional `~/.config/shiver/config.toml` for settings earlier specs fixed.
Human-facing overview: [onepager.html](onepager.html).

## Settings

| Key | Type | Default | Effect |
|---|---|---|---|
| `packs_dir` | path | `<config dir>/shiver/packs` | Packs folder; `~/` is expanded. |
| `disabled_packs` | list of folder names | `[]` | Pack folders skipped when loading. |
| `sound.enabled` | bool | `true` | `false` acts like `--no-sound` everywhere. |
| `sound.volume` | number 0.0–1.0 | `1.0` | Passed to `pw-play --volume=V` or `paplay --volume=round(V*65536)`. |
| `notify.default_icon` | bool | `true` | `false` turns off the Signal Flame fallback (0009). |
| `cooldowns.enabled` | bool | `true` | `false` ignores and doesn't write the history (0007). |
| `cooldowns.history` | integer 1–1000 | `50` | Picks kept in the history. |
| `context.morning` | `HH:MM-HH:MM` | `05:00-11:00` | Window for `morning` (0008). |
| `context.late` | `HH:MM-HH:MM` | `22:00-05:00` | Window for `late`; may cross midnight. |
| `context.boost` | number ≥ 1.0 | `3.0` | Weight multiplier for active context tags. |
| `hook.min_gap` | duration | `10m` | Default for `on-fail --min-gap` (0010). |
| `hook.ignore_commands` | list of names | `[]` | Added to the built-in ignored commands. |

## Requirements

### R1: Loading

The file is `--config FILE` (global option), else `SHIVER_CONFIG` (if
non-empty), else `<config dir>/shiver/config.toml`. A missing default file means
all defaults; a missing file given by `--config` or `SHIVER_CONFIG` is an error.
Invalid TOML, an unknown key, a wrong type or an out-of-range value is an error:
`shiver: invalid config <path>: <details>` (the details name the key or line),
exit 1.

**Acceptance:**
- [x] Test `config::tests::defaults` passes (empty file equals the table).
- [x] Test `config::tests::parses_everything` passes.
- [x] Test `config::tests::rejects_unknown_keys` passes.
- [x] Test `config::tests::rejects_bad_values` passes (volume 1.5, history 0,
      boost 0.5, bad window, bad duration).
- [x] Test `cli::config_errors_name_the_file` passes.
- [x] Test `cli::missing_explicit_config_is_an_error` passes.

### R2: Precedence

`--packs-dir` > `SHIVER_PACKS_DIR` > `packs_dir` > default. `on-fail
--min-gap` > `hook.min_gap`; `shiver hook --min-gap` is written into the snippet
as before. `--no-sound` and `sound.enabled = false` both silence sound.

**Acceptance:**
- [x] Test `cli::config_packs_dir_and_precedence` passes.
- [x] Test `cli::config_hook_min_gap` passes (via `on-fail --dry-run`).

### R3: Effects

Each setting changes behaviour as in the table.

**Acceptance:**
- [x] Test `cli::config_disabled_packs` passes (`list` and `say` skip them).
- [x] Test `cli::config_cooldowns_off` passes (no history file written).
- [x] Test `cli::config_history_size` passes (`history = 2` keeps 2 entries).
- [x] Test `cli::config_context_windows` passes (`late = "12:00-13:00"` makes
      a `late` line pickable `--at 12:30` and not `--at 23:00`).
- [x] Test `select::tests::custom_boost` passes.
- [x] Test `sound::tests::passes_volume` passes (both players' arguments).
- [x] Test `cli::config_ignore_commands` passes (via `on-fail --dry-run`).
- [x] Test `notify::tests::default_icon_can_be_off` passes.

### R4: `shiver config`

Prints `# <path> (exists|not found; using defaults)` and then every setting in
effect as valid TOML (the format of the table above), so it can be copied into
the file. Exit 0. With an invalid file it fails as in R1.

**Acceptance:**
- [x] Test `cli::config_command_prints_effective_settings` passes, and its
      output (minus the first comment line) parses back to the same settings.

### R5: Docs

README: a "Configuration" section with the example file and the precedence
rules; the commands table lists `shiver config` and `--config`.

**Acceptance:**
- [x] Review: README updated.

## Out of scope

- Timer schedule in the config file; per-voice settings; live reload.
