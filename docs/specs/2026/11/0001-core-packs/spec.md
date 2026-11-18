---
id: 0001
name: core-packs
status: done
created: 2026-11-18
---

# 0001: Core packs and line selection

Set up the Cargo workspace and the `shiver-core` library: load packs from a
folder, merge their voices, and pick a weighted random line, optionally filtered
by voice and tag. Human-facing overview: [onepager.html](onepager.html).

## Pack format

`<packs-dir>/<folder>/pack.toml`:

```toml
[pack]
name = "example"            # optional, defaults to the folder name
license = "CC0-1.0"         # optional

[voices.resolve]            # voice key: "resolve"
display = "RESOLVE"         # optional, defaults to the key
color = "#8a5cd6"           # optional, "#rrggbb"
sound = "sounds/resolve.ogg"  # optional, default sound for this voice
icon = "icons/resolve.png"    # optional

[[lines]]
voice = "resolve"           # required
text = "Stand up."          # required, non-empty after trimming
tags = ["late", "idle"]     # optional, default []
weight = 1.0                # optional, default 1.0, finite and > 0
sound = "sounds/other.ogg"  # optional, overrides the voice's sound
```

Unknown keys are ignored. Relative paths resolve against the folder of the
`pack.toml` they appear in; absolute paths are used as-is.

## Requirements

### R1: Workspace

A Cargo workspace at the repo root with `crates/shiver-core` as a library crate
(edition 2024). The binary crate is out of scope.

**Acceptance:**
- [x] `cargo build --workspace` succeeds.
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test` pass.

### R2: Parse a pack file

Parse a `pack.toml` into a pack with its voices and lines, applying the defaults
above.

**Acceptance:**
- [x] Test `pack::tests::parses_full_pack` passes: every field from the example
      above is read.
- [x] Test `pack::tests::applies_defaults` passes: name defaults to the folder
      name, display to the voice key, tags to empty, weight to 1.0.
- [x] Test `pack::tests::ignores_unknown_keys` passes.

### R3: Load a library from a packs folder

`Library::load(dir)` reads every immediate subfolder of `dir`, in byte order of
the folder name, and returns the library plus a list of warnings. Each warning
names the file it is about and says what was wrong, readable through `Display`.

- `dir` doesn't exist: empty library, no warnings.
- `dir` exists but can't be read: an error.
- A subfolder without `pack.toml`: skipped, with a warning.
- Files directly in `dir`: ignored silently.

**Acceptance:**
- [x] Test `library::tests::loads_packs_in_folder_order` passes.
- [x] Test `library::tests::missing_dir_is_empty` passes.
- [x] Test `library::tests::warns_on_folder_without_pack_toml` passes.

### R4: Skip what's broken, keep the rest

Each case below skips only the broken item, adds one warning, and loading
continues:

| Case | Skipped |
|---|---|
| `pack.toml` isn't valid TOML or is missing a required field | the whole pack |
| a line's `voice` isn't defined by any loaded pack | that line |
| a line's `text` is empty or whitespace | that line |
| a line's `weight` is not finite or is ≤ 0 | that line |
| a voice's `color` isn't `#rrggbb` | that colour (voice keeps no colour) |
| a `sound` or `icon` file doesn't exist | that sound or icon |

**Acceptance:**
- [x] Test `library::tests::skips_malformed_pack` passes: the other packs load.
- [x] Test `library::tests::skips_line_with_unknown_voice` passes.
- [x] Test `library::tests::skips_empty_text` passes.
- [x] Test `library::tests::skips_invalid_weight` passes (covers 0, negative, NaN).
- [x] Test `library::tests::drops_invalid_color` passes.
- [x] Test `library::tests::drops_missing_sound_and_icon` passes.
- [x] Each of these tests also asserts exactly one warning naming the file.

### R5: Shared voices

Voices from all packs form one pool keyed by voice key. A line may use a voice
defined in any loaded pack. If more than one pack defines the same key, the
first in folder order wins and each later definition adds a warning.

**Acceptance:**
- [x] Test `library::tests::line_uses_voice_from_other_pack` passes.
- [x] Test `library::tests::first_voice_definition_wins` passes.

### R6: Pick a line

`Library::pick(&filter, &mut rng)` returns one matching line as an
`Interjection`, or `None` if no line matches. The filter has an optional voice
key and an optional tag; a line matches if it has that voice and contains that
tag. The RNG is passed in so tests can seed it.

Each matching line is picked with probability proportional to its weight.

The `Interjection` holds the line's text, its voice (key, display name,
optional colour), the sound (the line's own if set, else the voice's) and the
icon (the voice's).

**Acceptance:**
- [x] Test `select::tests::filters_by_voice` passes.
- [x] Test `select::tests::filters_by_tag` passes.
- [x] Test `select::tests::filters_by_voice_and_tag` passes.
- [x] Test `select::tests::no_match_returns_none` passes (also for an unknown
      voice key and for an empty library).
- [x] Test `select::tests::same_seed_same_pick` passes.
- [x] Test `select::tests::respects_weights` passes: with two lines weighted 1
      and 9, 10,000 seeded picks choose the heavier one between 85% and 95% of
      the time.
- [x] Test `select::tests::line_sound_overrides_voice_sound` passes, and the
      voice sound is used when the line has none.

### R7: Pack summaries

The library lists its loaded packs in folder order, each with its name, folder
path, the keys of the voices it defined (after merging), and its number of
loaded lines. This is what `shiver list` will show.

**Acceptance:**
- [x] Test `library::tests::summarises_packs` passes.

### R8: No desktop I/O

`shiver-core` only reads the packs folder and checks whether asset files exist.
It doesn't print, play sound, send notifications, spawn processes or look up
config paths itself.

**Acceptance:**
- [x] `shiver-core`'s dependencies are limited to `serde`, `toml`, `rand` and
      `thiserror` (plus `tempfile` as a dev-dependency if needed).
- [x] No `print!`, `println!`, `eprintln!` or `dbg!` in `shiver-core/src`.

### R9: Original test content

Test packs and fixtures are original text written for these tests.

**Acceptance:**
- [x] Review: no fixture contains text, sounds or images from another work.

## Out of scope

- The `shiver` binary, commands and output.
- Locating `~/.config/shiver/packs`.
- Notifications and sound playback.
- Cooldowns, context-aware selection and a config file.
- Watching packs for changes.
