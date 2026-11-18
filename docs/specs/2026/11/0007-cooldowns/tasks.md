# 0007: Cooldowns – tasks

- [x] 1. `line_id` (FNV-1a) and `Line::id()` in shiver-core (R1)
- [x] 2. `Seen` and `Chorus::pick_with_history` with fresh lines, oldest-half fallback and voice turns (R2)
- [x] 3. `Interjection` carries its line id
- [x] 4. `history` module in the binary: parse, render, load with warnings, save last 50 atomically (R3)
- [x] 5. `say` and random `notify` read and record history; one-off doesn't (R3)
- [x] 6. Test isolation: state dir per test (R4)
- [x] 7. README (R5)
- [x] 8. All checks pass; tick acceptance criteria

## Notes

- Approved by the agent under the user's delegation ("if the specs look good,
  auto-approve, implement and commit"), 2026-11-18.
- Changed before coding: `cli::one_off_notify_skips_history` became a review
  item, because the test would send a real notification (or fail on CI without
  D-Bus).
- `Interjection` gained an `id` field; one-off messages get
  `line_id(voice, text)` too, though they're never recorded.
- CLI tests: each `Packs` sandbox has its own `XDG_STATE_HOME`; other tests use
  a shared temp state folder. Confirmed `~/.local/state/shiver` wasn't created
  by the test run.
- `say_same_seed_same_line` now clears the history between its two runs: with
  cooldowns, the same seed gives the same line only for the same history.
- Extra tests: `history::tests::missing_file_is_empty`, `record_appends`,
  `cli::corrupt_history_only_warns`.
- 126 tests; coverage 96.56% (minimum stays 96%).
