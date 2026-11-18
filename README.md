<div align="center">

# shiver

Inner-voice popups for the desktop.

[![CI](https://img.shields.io/github/actions/workflow/status/ngenethlis/shiver/ci.yml?branch=main&label=CI&logo=github)](https://github.com/ngenethlis/shiver/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.89%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

[Features](#features) – [Installation](#installation) – [Quick Start](#quick-start) – [Packs](#packs) – [Commands](#commands) – [Roadmap](#roadmap) – [Development](#development) – [License](#license)

</div>

`shiver` shows a short line attributed to a named "voice" as a desktop
notification, optionally with a sound.

It's inspired by the skill interjections in **Disco Elysium**, but the tool itself is
generic: every voice, line and sound comes from packs that you write.

> **Status:** early

## Features

- **Your own voices:** voices, lines, colours, icons and sounds come from plain
  TOML packs.
- **Terminal:** `shiver say` prints a line in a speech box, with the voice name
  in its colour.
- **Desktop notifications:** `shiver notify` works with any freedesktop
  notification server (COSMIC, KDE Plasma, GNOME, ...) and plays the voice's
  sound with PipeWire or PulseAudio.
- **Weighted picks:** make lines rarer or more common, and narrow the pick by
  voice or tag.
- **Forgiving:** a broken pack or line is skipped with a warning; everything
  else still works.
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

Make a pack:

```sh
mkdir -p ~/.config/shiver/packs/mine
cat > ~/.config/shiver/packs/mine/pack.toml <<'EOF'
[voices.drift]
display = "DRIFT"
color = "#8a5cd6"
sound = "/usr/share/sounds/freedesktop/stereo/message-new-instant.oga"

[[lines]]
voice = "drift"
text = "Water the plant."
tags = ["idle"]
EOF
```

Then:

```console
$ shiver say
 DRIFT
╭──────────────────╮
│ Water the plant. │
╰──────────────────╯

$ shiver notify       # desktop notification + sound
```

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

Use another packs folder with `--packs-dir DIR` or `SHIVER_PACKS_DIR`.
Colour is turned off when output isn't a terminal, or with `NO_COLOR=1`.

## Roadmap

- **v0:** CLI, stdout and desktop notifications, sound via PipeWire/PulseAudio.
  Still to come: a systemd timer for random popups and an example pack.
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
