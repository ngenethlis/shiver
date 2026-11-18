# 0001: Core packs and line selection – tasks

- [x] 1. Create the Cargo workspace and the `shiver-core` crate with its dependencies (R1, R8)
- [x] 2. Add `Warning` and `LoadError` types (R3)
- [x] 3. Parse `pack.toml` into `Pack`, `Voice` and `Line` with defaults and path resolution (R2)
- [x] 4. Load packs from a folder in folder order, skipping folders without `pack.toml` and malformed packs (R3, R4)
- [x] 5. Merge voices across packs, first wins; validate colours and asset files (R4, R5)
- [x] 6. Load lines: skip unknown voice, empty text, invalid weight; drop missing sounds (R4, R5)
- [x] 7. Pack summaries (R7)
- [x] 8. Weighted `pick` with voice/tag filter returning `Interjection` (R6)
- [x] 9. Check fixtures are original, no printing in core, deps limited (R8, R9)
- [x] 10. `fmt`, `clippy`, `test` clean; tick acceptance criteria in `spec.md` (R1)

## Notes

- rand is 0.10: `Rng` is the core trait (re-exported from `rand_core`),
  `RngExt` has the helper methods, weighted picking is
  `rand::distr::weighted::WeightedIndex`, seeded RNG is `rand::rngs::StdRng`.
- `drops_missing_sound_and_icon` covers three dropped assets in one pack, so it
  asserts three warnings (one per dropped item), all naming the `pack.toml`.
- Extra tests beyond the spec: `skips_invalid_toml`, `unreadable_dir_is_an_error`,
  `keeps_valid_color`, `keeps_existing_sound`, `keeps_absolute_paths`,
  `no_filter_matches_everything`, `interjection_carries_voice`,
  `warning_display_names_file`.
- Next spec: the `shiver` binary (`say`, `notify`, `list`), which finds
  `~/.config/shiver/packs` and prints the load warnings to stderr.
