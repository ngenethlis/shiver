# 0011: Config file – tasks

- [x] 1. Core: `ContextRules` (windows, boost) and skipping pack folders
- [x] 2. `config` module: parse, validate, defaults, render as TOML (R1, R4)
- [x] 3. Load the config first; packs dir precedence (R1, R2)
- [x] 4. Wire each setting: disabled packs, sound, volume, default icon, cooldowns, history size, context, hook (R3)
- [x] 5. `shiver config` and `--config` / `SHIVER_CONFIG` (R1, R4)
- [x] 6. Tests (R1–R4)
- [x] 7. README (R5)
- [x] 8. All checks pass; tick acceptance criteria

## Notes

- Approved by the agent under the user's delegation, 2026-11-18.
- Core: `ContextRules` (windows + boost; `Context` now carries its boost) and
  `Chorus::load_skipping`. Extra core tests: `context::tests::custom_windows`,
  `chorus::tests::skips_disabled_folders`.
- `Notify` and `Sound` became structs carrying `default_icon` and `volume`;
  `history::KEEP` became a parameter.
- CLI tests now also set `XDG_CONFIG_HOME` and remove `SHIVER_CONFIG`, so a real
  `~/.config/shiver/config.toml` can't affect them.
- `notify::tests::default_icon_can_be_off` is shallow (the switch lives in
  `emit`, which needs a desktop); `cli::notify_reports_unreachable_notification_server`
  covers the icon being installed when it's on.
- Extra tests: `config::tests::expands_home`, `round_trips_through_toml`.
- 176 tests; coverage 97.55% (minimum 97%).
