# 0004: Default pack and shiver init – tasks

- [x] 1. Write the default pack and its LICENSE (R1)
- [x] 2. Add `Library::lines()` to shiver-core so the pack's lines can be checked
- [x] 3. `init` module: embed the pack, write it, `--force` (R2)
- [x] 4. `shiver init` command, handled before loading the library (R2)
- [x] 5. "run shiver init" hint for `say`, `notify` and `list` (R3)
- [x] 6. Tests (R1–R3)
- [x] 7. README Quick Start (R4)
- [x] 8. All checks pass; tick acceptance criteria

## Notes

- Added `Library::lines()` to shiver-core (not in the spec) so tests can check
  the pack's voices and tags without parsing TOML in the binary.
- The pack is embedded with `include_str!("../packs/default/pack.toml")`, so it
  lives inside the `shiver` crate and ships with `cargo install`.
- Default-pack tests ignore "sound file not found" warnings: CI runners may not
  have `sound-theme-freedesktop`. `default_pack_sounds_are_system_sounds` checks
  the paths instead.
- Extra tests: `init_ignores_broken_packs`, `install_reports_created_then_replaced`.
- Coverage after this spec: 95.50% lines (minimum 95%).
