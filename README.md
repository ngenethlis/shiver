<div align="center">

# shiver

Inner-voice popups for the desktop.

[![CI](https://img.shields.io/github/actions/workflow/status/ngenethlis/shiver/ci.yml?branch=main&label=CI&logo=github)](https://github.com/ngenethlis/shiver/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.89%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

[Features](#features) – [Installation](#installation) – [Quick Start](#quick-start) – [Packs](#packs) – [Commands](#commands) – [Random Popups](#random-popups) – [Roadmap](#roadmap) – [Development](#development) – [License](#license)

</div>

`shiver` shows a short line attributed to a named "voice" as a desktop
notification, optionally with a sound.

It's inspired by the skill interjections in **Disco Elysium**, but the tool itself is
generic: every voice, line and sound comes from packs that you write.

> **Status:** early

## Features

- **Your own voices:** voices, lines, colours, icons and sounds come from plain
  TOML packs.
- **Starter pack:** `shiver init` installs five original voices with about 30
  lines to start from.
- **Terminal:** `shiver say` prints a line in a speech box, with the voice name
  in its colour.
- **Desktop notifications:** `shiver notify` works with any freedesktop
  notification server (COSMIC, KDE Plasma, GNOME, ...) and plays the voice's
  sound with PipeWire or PulseAudio.
- **Weighted picks:** make lines rarer or more common, and narrow the pick by
  voice or tag.
- **Forgiving:** a broken pack or line is skipped with a warning; everything
  else still works.
- **Random popups:** a systemd user timer lets voices interject on their own
  through the day.
- **Scriptable:** send one-off messages with `--text`, no pack needed.

## Installation

Needs Linux, Rust 1.89+, and `pw-play` or `paplay` for sound.

```sh
cargo install --git https://github.com/ngenethlis/shiver shiver
```

Or from a clone:

```sh
cargo install --path crates/shiver
```

## Quick Start

Install the default pack, then ask for a line:

```console
$ shiver init
created ~/.config/shiver/packs/default/pack.toml
try `shiver say` or `shiver notify`; edit the pack to make it yours

$ shiver say --voice lantern
 LANTERN
╭──────────────────────────────────────────────────────────────╮
│ The bug will still be there tomorrow. You'll be better at it │
│ then.                                                        │
╰──────────────────────────────────────────────────────────────╯

$ shiver notify --tag break    # desktop notification + sound
```

The default pack has five voices (BACKBONE, CLOCKWORK, STATIC, LANTERN, SIGNAL)
and the tags `morning`, `focus`, `break`, `late` and `win`. Edit
`~/.config/shiver/packs/default/pack.toml` to change it, or add packs of your own
next to it.

## Packs

A pack is a folder in `~/.config/shiver/packs/` with a `pack.toml`. Relative
paths are resolved from the pack's folder.

```toml
[pack]
name = "desk"                  # optional, defaults to the folder name
license = "CC0-1.0"            # optional

[voices.drift]                 # the voice key lines refer to
display = "DRIFT"              # optional, defaults to the key
color = "#8a5cd6"              # optional, #rrggbb
sound = "sounds/drift.ogg"     # optional, default sound for this voice
icon = "icons/drift.png"       # optional

[[lines]]
voice = "drift"
text = "Your tea went cold an hour ago."
tags = ["late", "idle"]        # optional
weight = 2.0                   # optional, default 1.0; higher = more often
sound = "sounds/other.ogg"     # optional, overrides the voice's sound
```

Voices are shared between packs, so one pack can add lines for a voice defined
in another. If two packs define the same voice, the first (by folder name) wins.

Run `shiver list` to see what's loaded. Problems are printed as warnings.

## Commands

| Command | What it does |
|---|---|
| `shiver say [--voice V] [--tag T]` | Print a random line in the terminal |
| `shiver notify [--voice V] [--tag T] [--no-sound]` | Show a random line as a notification, with sound |
| `shiver notify --voice V --text "..." [--sound FILE]` | Show a one-off message |
| `shiver list` | List packs, voices and line counts |
| `shiver init [--force]` | Install the default pack |
| `shiver timer enable \| disable \| status` | Random popups through the day ([below](#random-popups)) |

Use another packs folder with `--packs-dir DIR` or `SHIVER_PACKS_DIR`.
Colour is turned off when output isn't a terminal, or with `NO_COLOR=1`.

## Random Popups

Let the voices interject on their own with a systemd user timer:

```console
$ shiver timer enable
wrote ~/.config/systemd/user/shiver.service
wrote ~/.config/systemd/user/shiver.timer
schedule: every 45m between 09:00-21:00, jitter 20m (popups 09:00-20:35)
timer enabled; next popup: Wed 2026-11-18 10:47:00 GMT

$ shiver timer status     # schedule and next popup
$ shiver timer disable    # stop and remove the timer
```

By default a popup comes about every 45 minutes between 9:00 and 21:00. Change
it with:

| Option | Default | What it does |
|---|---|---|
| `--every` | `45m` | Time between popups, e.g. `30m`, `1h`, `1h30m` (at least `5m`) |
| `--between` | `09:00-21:00` | Daily window; `00:00-24:00` for all day, `22:00-02:00` crosses midnight |
| `--jitter` | `20m` | Each popup comes at a random point up to this long after its slot; `0m` for exact times |

Run `enable` again to change the schedule. Because of the jitter, the last
popup can come a little after the window ends; `enable` warns you if it will.
Missed popups (computer asleep) are skipped, not caught up.

The timer runs the binary that ran `enable`, and uses `--packs-dir` or
`SHIVER_PACKS_DIR` if you gave one.

<details>
<summary>Setting up the timer by hand</summary>

Example units are in [`contrib/systemd/`](contrib/systemd/). They expect the
binary at `~/.cargo/bin/shiver`; edit `ExecStart` if it's elsewhere.

```sh
mkdir -p ~/.config/systemd/user
cp contrib/systemd/shiver.service contrib/systemd/shiver.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now shiver.timer
systemctl --user list-timers shiver.timer
```

</details>

## Roadmap

- **v0:** CLI, stdout and desktop notifications, sound via PipeWire/PulseAudio,
  a default pack and random popups with a systemd timer.
- **v1:** context-aware selection (time of day, idle, failed shell command)
  and cooldowns.
- **v2:** a custom Wayland layer-shell overlay and an optional daemon.

## Development

Work is spec-driven: each change starts as a spec in [`docs/specs/`](docs/specs/),
and [`AGENTS.md`](AGENTS.md) describes the workflow and coding guidelines. The
overall architecture is in [`docs/design.html`](docs/design.html).

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo xtask check-specs
```

CI also checks coverage (`cargo llvm-cov`) and dependency licenses (`cargo deny`).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
