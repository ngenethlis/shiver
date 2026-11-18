# Backlog

Ideas and deferred work that don't have a spec yet. When starting one, turn it
into a spec under `docs/specs/` (see `AGENTS.md`) and remove it from here.
Rough priority within each section: top first.

## Next (v1: context)

- **Context-aware picks:** choose tags from the situation automatically, e.g.
  `morning`/`late` from the time of day, `idle` after a stretch of inactivity.
- **Shell hook:** a zsh `precmd` example (and bash equivalent) that runs
  `shiver notify --tag fail` when a command exits non-zero.
- **Cooldowns:** don't repeat a line (or a voice) that was shown recently. Needs
  a small state file, e.g. under `~/.local/state/shiver/`.
- **Config file:** `~/.config/shiver/config.toml` for enabled packs, default
  output and volume.

## Timer

- Per-weekday schedules (e.g. weekdays only, or different hours at weekends).
- Support for systems without systemd.

## Content

- Original sound files and icons for the default pack (CC0), so it doesn't rely
  on system sounds being installed.
- More built-in packs to choose from in `shiver init`.
- A way to update an installed default pack after an upgrade without losing
  local edits (e.g. `shiver init --diff`).

## Output

- `shiver say` adapting the box width to the terminal.
- Notification urgency, timeout and actions; markup where the server supports it.

## v2: overlay and daemon

- A game-style popup as a Wayland layer-shell surface (coloured voice name,
  serif font, fade-out): `libcosmic`/`iced` or GTK4 with `gtk4-layer-shell`,
  in its own crate or behind a feature. Fall back to a notification on GNOME.
- An optional daemon for random intervals and event triggers, if timers and
  hooks stop being enough.
- Playing sound in-process (e.g. `rodio`) for a self-contained binary.

## Tooling and release

- Publish to crates.io (`shiver` was free as of 2026-11-17) and add release
  builds or distro packages. Before publishing, the default icon must move
  into the `shiver` crate (or be copied in by a build step): it's embedded from
  `assets/logo/`, outside the crate, which `cargo package` won't include.
- A CI job that builds on the minimum Rust version (1.89) to keep
  `rust-version` honest.
- Spec checker: verify that tests named in acceptance criteria exist.
- Spell-checking (`typos`) and hosted coverage history (Codecov), if wanted.
