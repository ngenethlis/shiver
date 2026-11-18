# 0002: The shiver command – tasks

- [x] 1. Create the `crates/shiver` binary crate with its dependencies (R1)
- [x] 2. `Output` trait and `warn` helper (R9)
- [x] 3. CLI arguments with clap: global `--packs-dir`/`SHIVER_PACKS_DIR` and hidden `--seed`; `say`, `notify`, `list` (R2, R3, R6)
- [x] 4. Load the library, print warnings, pick with clear errors (R2, R3)
- [x] 5. Speech box: wrapping, wide characters, colour (R4)
- [x] 6. Notification output and one-off interjections (R5, R6)
- [x] 7. Sound output with `pw-play` → `paplay` fallback, detached (R7)
- [x] 8. `list` output (R8)
- [x] 9. End-to-end tests in `tests/cli.rs` (R2, R3, R6, R8)
- [x] 10. `fmt`, `clippy`, `test` clean; tick automated criteria (R1)
- [x] 11. User runs the manual notification checks on COSMIC (R5, R6)

## Notes

- notify-rust 4.19 defaults to the `z` feature (zbus, pure Rust); the `d`
  feature would need libdbus headers, so don't enable it.
- rand 0.10: `rand::make_rng::<StdRng>()` for an entropy-seeded RNG.
- clap's `env` support rejects an empty `SHIVER_PACKS_DIR` with a usage error,
  so the variable is read in `run()` and an empty value counts as unset.
- `list` printing lives in `main.rs`: the `Output` trait is for showing an
  interjection, and `list` isn't one.
- Integration tests remove `SHIVER_PACKS_DIR` and `NO_COLOR` from the
  environment so the developer's settings don't affect them.
