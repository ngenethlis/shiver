# shiver

Inner-voice popups for the desktop.

`shiver` shows a short line attributed to a named "voice" as a desktop
notification, optionally with a sound.

It's inspired by the skill interjections in **Disco Elysium**, but the tool itself is
generic: every voice, line and sound comes from packs that you write.

> **Status:** early

## Platform

Linux with a freedesktop notification server (COSMIC, KDE Plasma, GNOME, ...).
Sound is played with `pw-play`, falling back to `paplay`.

## Roadmap

- **v0:** CLI, stdout and desktop notifications, sound via PipeWire/PulseAudio.
- **v1:** context-aware selection (time of day, idle, failed shell command),
  weighted lines and cooldowns.
- **v2:** a custom Wayland layer-shell overlay and an optional daemon.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
