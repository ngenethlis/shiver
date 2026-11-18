# 0010: Shell hook for failed commands – tasks

- [x] 1. `fail` as a never-active context tag in shiver-core (R1)
- [x] 2. `hook` module: ignore rules, rate limit, `last-fail` state, snippets (R2, R3)
- [x] 3. `shiver on-fail` and `shiver hook` commands (R2, R3)
- [x] 4. Default pack `fail` lines (R4)
- [x] 5. Tests, including snippets run in real zsh and bash (R1–R4)
- [x] 6. README (R5)
- [x] 7. All checks pass; tick acceptance criteria
- [ ] 8. User tries the hook in their shell (R5)

## Notes

- Approved by the agent under the user's delegation, 2026-11-18.
- Changed during implementation: the bash snippet first used a `DEBUG` trap,
  but its test showed the trap also catches non-user commands (it recorded the
  snippet's own `PROMPT_COMMAND=` line; in a real shell it would catch later
  `.bashrc` lines and other prompt commands). It now reads `history 1` at each
  prompt and fires only when the history number changed. Spec R3 and the
  one-pager were updated to match.
- Coverage first fell to 95.85% (below the 96% minimum) because the notify and
  on-fail paths only ran against a real desktop. CLI tests now set
  `DBUS_SESSION_BUS_ADDRESS` to a missing socket, so those paths run up to the
  failed send without ever reaching the real desktop, and `XDG_DATA_HOME` to a
  temp folder for the icon. Coverage 97.15%; CI minimum raised to 97%.
- Extra tests: `hook::tests::reads_and_writes_last_fail`, `quotes_for_the_shell`,
  `cli::notify_reports_unreachable_notification_server`,
  `cli::on_fail_reports_unreachable_notification_server`.
- 155 tests.
