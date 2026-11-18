---
id: 0010
name: shell-hook
status: done
created: 2026-11-18
---

# 0010: Shell hook for failed commands

`shiver hook zsh|bash` prints a snippet that, after a failed command, runs
`shiver on-fail` in the background; `on-fail` decides whether to show a `fail`
line. Human-facing overview: [onepager.html](onepager.html).

## Requirements

### R1: `fail` is an event tag

`fail` is a context tag (0008) that's never active on its own: lines tagged
`fail` are only picked with an explicit `--tag fail`.

**Acceptance:**
- [x] Test `context::tests::fail_is_never_active` passes (at every hour).
- [x] Test `select::tests::fail_lines_need_explicit_tag` passes.

### R2: `shiver on-fail`

`shiver on-fail --exit-code N [--command CMD] [--min-gap D] [--no-sound]
[--dry-run]`:

1. **Ignored exit codes:** 0, 130 (Ctrl-C), 141 (broken pipe), 148 (Ctrl-Z).
2. **Ignored commands:** the first word of `CMD`, after skipping leading
   `VAR=value` assignments, `sudo`, `command` and `time`, is one of `grep`,
   `egrep`, `fgrep`, `rg`, `ag`, `diff`, `cmp`, `test`, `[`, `[[`, `false`,
   `which`, `type`.
3. **Rate limit:** if the last hook popup was less than `--min-gap` ago
   (default `10m`, duration format as in 0006), do nothing. The time is kept in
   `<state dir>/shiver/last-fail` (unix seconds).
4. Otherwise pick a line with `--tag fail` and the usual history and context,
   show it like `notify` (with sound unless `--no-sound`), record it in the
   history, and write `last-fail`.

`on-fail` never prints anything and always exits 0 when it decides not to fire,
or when there are no packs or no `fail` lines. With `--dry-run` it prints what
it decided instead of notifying (`skip: ignored exit code 130`,
`skip: ignored command grep`, `skip: last popup 3m ago (min gap 10m)`,
`skip: no fail lines`, or `would notify: <VOICE>: <text>`) and doesn't write
`last-fail` or the history.

**Acceptance:**
- [x] Test `hook::tests::ignores_exit_codes` passes.
- [x] Test `hook::tests::ignores_commands` passes (including `sudo grep`,
      `LC_ALL=C grep`, `time diff`; and `cargo` is not ignored).
- [x] Test `hook::tests::rate_limit` passes (fires at exactly the gap, not
      before).
- [x] Test `cli::on_fail_dry_run` passes for each `skip:` reason and
      `would notify:`.
- [x] Test `cli::on_fail_is_silent_without_packs` passes (no output, exit 0).

### R3: `shiver hook zsh|bash`

Prints a snippet for `eval` that:

- knows which command just ran (zsh: `preexec` via `add-zsh-hook`; bash: at
  each prompt, the newest history entry, counted as new only if its history
  number changed),
- after it runs, if the exit code is non-zero and a command was actually run
  (not an empty prompt), starts `<EXE> on-fail --exit-code <code> --command
  <cmd> [options]` in the background with all output discarded, without job
  control messages,
- uses the canonical path of the running binary, single-quoted for the shell,
- passes on `--min-gap` and `--no-sound` if given to `hook`.

**Acceptance:**
- [x] Test `cli::hook_zsh_snippet_runs` passes: in a real `zsh` (skipped if not
      installed), with the snippet's binary swapped for a script that logs its
      arguments, a failing `false` command logs
      `on-fail --exit-code 1 --command false`, and a successful command and an
      empty prompt log nothing.
- [x] Test `cli::hook_bash_snippet_runs` passes: the same in `bash`.
- [x] Test `cli::hook_passes_options` passes.
- [x] Test `cli::hook_rejects_unknown_shell` passes (exit 2).

### R4: Default pack lines

The default pack gains at least 6 original lines tagged `fail`, spread over at
least 3 voices. The default-pack tag test (0004) accepts `fail`.

**Acceptance:**
- [x] Test `init::tests::default_pack_uses_known_tags` passes with `fail`.
- [x] Review: the new lines are original.

### R5: Docs

README: a "Shell hook" section (setup line for zsh and bash, what's ignored,
the rate limit, `--dry-run`, the bash history caveat), and the commands
table.

**Acceptance:**
- [x] Review: README updated.
- [x] Manual: with the hook in the user's shell, a failing command (e.g.
      `cargo build` in an empty folder) shows a `fail` popup.

## Out of scope

- Configurable ignore lists (spec 0011).
- Other shells; success hooks; escalating lines for repeated failures.
