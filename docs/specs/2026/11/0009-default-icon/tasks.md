# 0009: Default notification icon – tasks

- [x] 1. `icon` module: embed the 128 px icon, `ensure(data_dir)` (R1, R2)
- [x] 2. Notifications fall back to it (R3)
- [x] 3. Tests (R1–R3)
- [x] 4. `assets/README.md` and backlog (R4)
- [x] 5. All checks pass; tick acceptance criteria
- [x] 6. User checks the icon on COSMIC (R3)

## Notes

- Added at the user's request ("use Signal Flame as the default"), between the
  v1 specs; the shell hook moves to 0010 and the config file to 0011.
- Approved by the agent under the user's delegation, 2026-11-18.
- `icon::default_icon()` (which reads the real `dirs::data_dir()`) isn't
  covered by tests; `ensure` is. Coverage 96.09%, above the 96% minimum.
- 140 tests.
- 2026-11-18: first desktop check showed no icon. notify-rust only sent the
  plain path as `app_icon`; the spec wants a `file://` URI there and COSMIC
  shows the `image-path` hint. Now both are sent as a `file://` URI
  (test `notify::tests::icons_are_file_uris`). Waiting for a re-check.
- 2026-11-18: re-checked on COSMIC after the fix: the Signal Flame shows.
