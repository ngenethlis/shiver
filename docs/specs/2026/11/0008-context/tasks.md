# 0008: Time-of-day context – tasks

- [x] 1. `context` module in shiver-core: context tags, windows, `Context::at` / `none` (R1)
- [x] 2. `Filter.context`; restrict context-tagged lines; boost active ones ×3 (R2, R3)
- [x] 3. Update existing tests that used `late` as an ordinary tag
- [x] 4. Binary: local time via `jiff`, `--at HH:MM` on `say` and `notify` (R4)
- [x] 5. Tests (R1–R4)
- [x] 6. README (R5)
- [x] 7. All checks pass (incl. `cargo deny`); tick acceptance criteria

## Notes

- Approved by the agent under the user's delegation, 2026-11-18.
- jiff 1.0: `TimeZone::try_system()` then `Timestamp::now().to_zoned_datetime(tz)`
  (`Zoned::now()` would silently fall back to UTC).
- `anyhow::Context` clashes with `shiver_core::Context`, so main.rs uses the
  full path.
- Test packs in `select.rs` used `late` as an ordinary tag; renamed to `dusk`.
- Extra tests: `context::tests::knows_context_tags`.
- 134 tests; coverage 96.54%.
