# 0006: Random popups with a systemd timer – tasks

- [x] 1. `Schedule`: parse durations and windows, validate, compute slots, describe, late warning (R8, R9)
- [x] 2. Render `shiver.service` and `shiver.timer`, with quoting (R1)
- [x] 3. `enable`, `disable`, `status` flows with an injectable `systemctl` runner (R2–R4)
- [x] 4. CLI wiring: `shiver timer enable|disable|status`, schedule errors exit 2 (R2–R4, R8)
- [x] 5. Example files in `contrib/systemd/` (R5)
- [x] 6. Unit tests and CLI tests with a fake `systemctl` (R1–R5, R8, R9)
- [x] 7. README section (R6)
- [x] 8. All checks pass; tick acceptance criteria
- [x] 9. User checks it for real on COSMIC (R7)

## Notes

- Spec corrected after approval, before coding: the R9 example used jitter 20m,
  which lands exactly on 21:00 and wouldn't warn (now 30m); the "at least one
  slot" rule can never fail and was removed; quoting and `OnCalendar` order
  were made precise.
- The window rule "holds at least one slot" is implied: offset 0 (the start)
  is always a slot.
- `OnCalendar` lines are ordered by minute (00, 15, 30, 45 for the default),
  which is what the spec's example file showed.
- Paths in `ExecStart`: `%`→`%%`, `$`→`$$`, and double quotes when needed.
  The contrib service keeps `%h` unescaped on purpose.
- CLI tests put a fake `systemctl` script first on `PATH` and point
  `XDG_CONFIG_HOME` at a temp folder, so the real user systemd is never touched.
- Verified with the real `systemd-analyze --user verify` and
  `systemd-analyze calendar` on files generated in a sandbox (default schedule
  and `--every 1h30m --between 22:00-02:00 --jitter 10m`).
- Coverage rose to 96.47%; CI minimum raised from 95% to 96%.
- Extra tests: `reads_schedule_back_from_timer_file`,
  `timer_enable_replaces_its_own_units`, `timer_status_custom_schedule`.
- 2026-11-18: checked on COSMIC with `--every 5m --between 00:00-24:00
  --jitter 0m`: the 01:10 slot fired (journal shows shiver.service started and
  finished), the notification arrived, and `disable` removed both units.
