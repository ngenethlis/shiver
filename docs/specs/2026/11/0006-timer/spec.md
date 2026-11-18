---
id: 0006
name: timer
status: done
created: 2026-11-18
---

# 0006: Random popups with a systemd timer

Run `shiver notify` on a schedule with a systemd user timer, set up either by
`shiver timer enable [--every D] [--between HH:MM-HH:MM] [--jitter D]` or by
copying example unit files. Human-facing overview:
[onepager.html](onepager.html).

## Unit files

`shiver.service` (`<EXEC>` is the binary path, `<ARGS>` is empty or
`--packs-dir <dir> `; paths are quoted for systemd as described in R1):

```ini
# Written by `shiver timer enable`. Remove with `shiver timer disable`.
[Unit]
Description=shiver: a random inner-voice popup

[Service]
Type=oneshot
ExecStart=<EXEC> <ARGS>notify
```

`shiver.timer`, shown for the default schedule (`--every 45m --between
09:00-21:00 --jitter 20m`):

```ini
# Written by `shiver timer enable`. Remove with `shiver timer disable`.
# Schedule: every 45m between 09:00-21:00, jitter 20m
[Unit]
Description=shiver: a random popup every 45m between 09:00-21:00

[Timer]
OnCalendar=*-*-* 09,12,15,18:00:00
OnCalendar=*-*-* 11,14,17,20:15:00
OnCalendar=*-*-* 10,13,16,19:30:00
OnCalendar=*-*-* 09,12,15,18:45:00
RandomizedDelaySec=20m
AccuracySec=1min
Persistent=false

[Install]
WantedBy=timers.target
```

## Requirements

### R1: Unit file rendering

A `timer` module renders both files exactly as above. In the binary path and
packs folder, `%` is written `%%` and `$` is written `$$` (systemd specifiers
and variables); a path containing whitespace, quotes or backslashes is wrapped
in double quotes with `"` and `\` escaped.

`OnCalendar` lines come from the schedule (R8): one line per distinct minute of
the slots, in ascending order of minute, listing the hours that have a slot at
that minute in ascending order.

**Acceptance:**
- [x] Test `timer::tests::renders_service` passes.
- [x] Test `timer::tests::renders_service_with_packs_dir` passes.
- [x] Test `timer::tests::quotes_paths_with_spaces` passes.
- [x] Test `timer::tests::renders_default_timer` passes (exact text above).
- [x] Test `timer::tests::renders_hourly_timer` passes: `--every 1h --between
      08:00-18:00 --jitter 0m` gives a single
      `OnCalendar=*-*-* 08,09,10,11,12,13,14,15,16,17:00:00` and
      `RandomizedDelaySec=0m`.
- [x] Manual: `systemd-analyze --user verify` reports no problems for the
      installed files.

### R2: `shiver timer enable [--force] [schedule options]`

1. Units folder: `<config dir>/systemd/user` (`~/.config/systemd/user`),
   created if missing.
2. `<EXEC>` is the canonical path of the running binary.
3. If `--packs-dir` or `SHIVER_PACKS_DIR` was given, it's written as
   `--packs-dir`; otherwise the service uses the default packs folder.
4. If either file exists and doesn't start with the `# Written by` line, fail
   with `shiver: <path> wasn't written by shiver; use --force to replace it`,
   exit 1, nothing written. Files written by shiver are replaced.
5. Write both files, print `wrote <path>` for each, then run
   `systemctl --user daemon-reload` and
   `systemctl --user enable --now shiver.timer`.
6. Print the schedule line (R9), any late-popup warning (R9), and
   `timer enabled; next popup: <time>`, taking the time from
   `systemctl --user show shiver.timer --property=NextElapseUSecRealtime --value`
   (printed as systemd gives it).

If `systemctl` is missing or fails, exit 1 with its error output; the written
files stay so the user can fix and retry.

**Acceptance:**
- [x] Test `cli::timer_enable_writes_units_and_starts_timer` passes (fake
      `systemctl` on `PATH` records its calls).
- [x] Test `cli::timer_enable_writes_packs_dir` passes.
- [x] Test `cli::timer_enable_refuses_foreign_units` passes.
- [x] Test `cli::timer_enable_force_replaces_foreign_units` passes.
- [x] Test `cli::timer_enable_reports_systemctl_failure` passes.

### R3: `shiver timer disable`

Run `systemctl --user disable --now shiver.timer` (a failure because the unit
isn't loaded is ignored), remove both files if they were written by shiver,
run `systemctl --user daemon-reload`, and print `timer disabled`. Files not
written by shiver are left alone with a warning. Running it when nothing is
installed succeeds.

**Acceptance:**
- [x] Test `cli::timer_disable_removes_units` passes.
- [x] Test `cli::timer_disable_when_not_installed` passes.
- [x] Test `cli::timer_disable_keeps_foreign_units` passes.

### R4: `shiver timer status`

If `shiver.timer` is active (`systemctl --user is-active shiver.timer`), prints
the schedule line (R9) from the `Schedule:` comment in the installed timer file
(`schedule: custom` if the file has none) and `timer enabled; next popup: <time>`.
Otherwise prints `timer not enabled`. Exit 0 either way.

**Acceptance:**
- [x] Test `cli::timer_status_enabled` passes, including the schedule line.
- [x] Test `cli::timer_status_not_enabled` passes.

### R5: Example files

`contrib/systemd/shiver.service` and `contrib/systemd/shiver.timer`, identical
to the rendered files with `<EXEC>` = `%h/.cargo/bin/shiver` and no packs
folder, except the first comment line says
`# Example unit for shiver; see the README. Or run \`shiver timer enable\`.`

**Acceptance:**
- [x] Test `timer::tests::contrib_files_match_rendered` passes.

### R6: README

A "Random popups" section shows `shiver timer enable`, `status` and `disable`,
describes the schedule and how to change it, and gives the manual steps for the
example files. `timer` is in the commands table; the roadmap no longer lists the
timer as to come.

**Acceptance:**
- [x] Review: README updated.

### R8: Schedule options

`shiver timer enable` takes:

| Option | Default | Meaning |
|---|---|---|
| `--every D` | `45m` | Time between slots. |
| `--between HH:MM-HH:MM` | `09:00-21:00` | Daily window. Slots start at the start time and repeat every `D` while before the end time. `24:00` is allowed as an end. If the end is earlier than the start, the window crosses midnight (e.g. `22:00-02:00`). |
| `--jitter D` | `20m` | Each popup fires at a random point up to `D` after its slot (`RandomizedDelaySec`). |

Durations are `<n>m`, `<n>h` or `<n>h<n>m`. Rules, each with a clear error
(exit 2):

- `--every` is at least `5m`.
- `--between` start and end differ (use `00:00-24:00` for all day).
- `--jitter` is less than `--every`, so popups stay in order.

(Every window holds at least one slot, its start, so no rule is needed for
that.)

Slots in a window that crosses midnight are written as times of day (a slot at
24:15 after the start becomes `00:15`); since the timer repeats daily, each
night's window is covered.

Durations are printed back in the shortest form (`90m` → `1h30m`).

**Acceptance:**
- [x] Test `timer::tests::parses_durations` passes.
- [x] Test `timer::tests::parses_window` passes.
- [x] Test `timer::tests::computes_slots` passes (default schedule gives the
      16 slots 09:00, 09:45, … 20:15).
- [x] Test `timer::tests::computes_slots_across_midnight` passes
      (`--every 45m --between 22:00-02:00` gives 22:00, 22:45, 23:30, 00:15,
      01:00, 01:45, and the timer file has the matching `OnCalendar` lines).
- [x] Test `timer::tests::rejects_invalid_schedules` passes (one case per rule).
- [x] Test `cli::timer_enable_with_schedule_options` passes.

### R9: Make late popups visible

The schedule line shows when popups can actually arrive, from the first slot
to the last slot plus the jitter:

```
schedule: every 45m between 09:00-21:00, jitter 20m (popups 09:00-20:35)
```

If the last slot plus the jitter is after the window's end, `enable` also
prints, to stderr, and still installs the timer:

```
shiver: warning: popups can arrive until 21:10, after the window ends at 21:00 (last slot 20:40 + jitter 30m); use a smaller --jitter or an earlier end
```

Times past midnight are shown as times of day.

**Acceptance:**
- [x] Test `timer::tests::describes_schedule` passes (default schedule gives
      the line above, and a window crossing midnight shows times of day).
- [x] Test `timer::tests::warns_when_popups_run_late` passes:
      `--every 50m --between 09:00-21:00 --jitter 30m` warns as above; the
      default schedule doesn't warn, and a popup ending exactly at the window's
      end doesn't warn.
- [x] Test `cli::timer_enable_warns_about_late_popups` passes: warning on
      stderr, exit 0, timer installed.

### R7: Works for real

**Acceptance:**
- [x] Manual: on COSMIC, after `shiver timer enable`, `shiver timer status`
      shows the next popup, and a notification arrives on its own.
- [x] Manual: `shiver timer disable` stops it and removes the files.
- [x] Manual: `systemd-analyze calendar` on the generated `OnCalendar` lines
      lists the expected slots.

## Out of scope

- Different schedules per weekday.
- Shell hooks, context-aware selection, a daemon.
- Systems without systemd.
