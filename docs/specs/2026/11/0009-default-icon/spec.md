---
id: 0009
name: default-icon
status: done
created: 2026-11-18
---

# 0009: Default notification icon

Use the Signal Flame logo as the notification icon for voices that don't have
their own. Human-facing overview: [onepager.html](onepager.html).

## Requirements

### R1: Embedded icon

`assets/logo/shiver-icon-128.png` is compiled into the `shiver` binary.

**Acceptance:**
- [x] Test `icon::tests::embedded_icon_is_png` passes (PNG signature, 128×128
      from the IHDR header).

### R2: Installing the icon

`icon::ensure(data_dir)` makes sure `<data_dir>/shiver/icons/shiver.png` holds
the embedded icon: it creates the folders, writes the file if it's missing or
its bytes differ, and leaves it alone if it matches. It returns the path. The
binary calls it with `dirs::data_dir()` (`~/.local/share`).

**Acceptance:**
- [x] Test `icon::tests::writes_icon` passes.
- [x] Test `icon::tests::keeps_matching_icon` passes (file not rewritten: its
      modification time is unchanged).
- [x] Test `icon::tests::replaces_different_icon` passes.

### R3: Fallback in notifications

When a notification's interjection has no icon, the default icon's path is
used. Voice icons still win. One-off `--text` notifications get it too. If no
data folder exists or writing fails: `shiver: warning: can't install the shiver
icon: ...` and the notification has no icon.

**Acceptance:**
- [x] Test `notify::tests::uses_default_icon_without_voice_icon` passes.
- [x] Test `notify::tests::voice_icon_beats_default` passes.
- [x] Manual: on COSMIC, `shiver notify` with the default pack shows the
      Signal Flame.

### R4: Docs

`assets/README.md` notes the icon is compiled into the binary. The backlog's
crates.io item notes the icon must move into the crate (or be copied by a build
step) before publishing.

**Acceptance:**
- [x] Review: both files updated.

## Out of scope

- Turning the default icon off.
- Desktop icon themes and `.desktop` files.
