---
id: 0004
name: default-pack
status: done
created: 2026-11-18
---

# 0004: Default pack and shiver init

Ship an original default pack inside the binary and add `shiver init` to write
it into the packs folder. Human-facing overview: [onepager.html](onepager.html).

## Requirements

### R1: The default pack

`crates/shiver/packs/default/pack.toml`, with a `LICENSE` file in the same
folder stating CC0-1.0, and `license = "CC0-1.0"` in the pack.

- Five voices: `backbone` (BACKBONE), `clockwork` (CLOCKWORK), `static`
  (STATIC), `lantern` (LANTERN), `signal` (SIGNAL), each with a distinct
  `#rrggbb` colour and a sound:

  | Voice | Sound (in `/usr/share/sounds/freedesktop/stereo/`) |
  |---|---|
  | backbone | `message.oga` |
  | clockwork | `bell.oga` |
  | static | `dialog-warning.oga` |
  | lantern | `dialog-information.oga` |
  | signal | `complete.oga` |

- At least 5 lines per voice, all original text written for this pack.
- Lines use only the tags `late`, `morning`, `break`, `focus`, `win`; each tag
  is used at least twice.

**Acceptance:**
- [x] Test `init::tests::default_pack_loads_cleanly` passes: the pack loads
      with no warnings other than missing sound files, has the five voices,
      and every voice has at least 5 lines.
- [x] Test `init::tests::default_pack_uses_known_tags` passes.
- [x] Review: all lines are original; the `LICENSE` file states CC0-1.0.

### R2: `shiver init`

`shiver init [--force]` writes the embedded pack to
`<packs-dir>/default/pack.toml`, creating folders as needed, using the same
packs folder rules as other commands (`--packs-dir`, `SHIVER_PACKS_DIR`,
default).

- Success: prints `created <path>` and on a second line
  `try \`shiver say\` or \`shiver notify\`; edit the pack to make it yours`,
  exit 0.
- `<packs-dir>/default/pack.toml` already exists and no `--force`:
  `shiver: <path> already exists; use --force to overwrite`, exit 1, file
  unchanged.
- With `--force`: overwrites and prints `replaced <path>` plus the same hint.

`init` doesn't load the library first, so broken packs don't stop it.

**Acceptance:**
- [x] Test `cli::init_creates_default_pack` passes: then `shiver list` shows
      the pack with 5 voices.
- [x] Test `cli::init_refuses_to_overwrite` passes.
- [x] Test `cli::init_force_overwrites` passes.

### R3: Hint when there are no packs

When `say` or `notify` find no packs, the error becomes
`shiver: no packs found in <dir>; run \`shiver init\` to install the default pack`.
`shiver list` with no packs adds the same hint on a second line.

**Acceptance:**
- [x] Test `cli::say_without_packs_fails` passes with the new message.
- [x] Test `cli::list_without_packs` passes with the new message.

### R4: README

The Quick Start uses `shiver init`, then `shiver say`, and shows a real
default-pack line. The Packs section still documents the format.

**Acceptance:**
- [x] Review: README updated; the shown output matches `shiver say` for that line.

## Out of scope

- Shipping sound files or icons.
- The systemd timer for random popups.
- More than one built-in pack, or updating an installed pack automatically.
