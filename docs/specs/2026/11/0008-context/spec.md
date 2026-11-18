---
id: 0008
name: context
status: done
created: 2026-11-18
---

# 0008: Time-of-day context

Make `morning` and `late` context tags tied to the local time: lines with them
are only picked in their window and are more likely then. Human-facing
overview: [onepager.html](onepager.html).

## Requirements

### R1: Context windows

`shiver-core` defines the context tags and their windows (start inclusive, end
exclusive, local time):

| Tag | Window |
|---|---|
| `morning` | 05:00–11:00 |
| `late` | 22:00–05:00 (crosses midnight) |

`Context::at(minutes_since_midnight)` gives the set of active context tags.
`Context::none()` has none active.

**Acceptance:**
- [x] Test `context::tests::windows` passes: 04:59 → late; 05:00 → morning;
      10:59 → morning; 11:00 → none; 21:59 → none; 22:00 → late; 00:00 → late.

### R2: Restrict

`Filter` gains a `context: Context`. A line with one or more context tags is
eligible only if at least one of them is active, or is the filter's explicit
`tag`. Lines without context tags are unaffected. This happens before the
cooldown rules of 0007.

**Acceptance:**
- [x] Test `select::tests::context_tags_restrict` passes: at noon, `late` and
      `morning` lines are never picked; untagged lines are.
- [x] Test `select::tests::explicit_tag_overrides_context` passes: `--tag late`
      at noon picks late lines.
- [x] Test `select::tests::no_context_means_no_context_lines` passes:
      with `Context::none()`, context-tagged lines only come with an explicit tag.

### R3: Boost

While a context tag is active, lines carrying it have their weight multiplied
by 3 (once, even if a line carries several active tags).

**Acceptance:**
- [x] Test `select::tests::active_context_boosts` passes: one `late` and one
      untagged line, both weight 1, at 23:00: over 10,000 picks (no history) the
      late line is chosen 70–80% of the time (expected 75%).

### R4: The binary

`say` and `notify` (not one-off `--text`) pass the context for the current
local time, read with `jiff`. A new option on both, `--at HH:MM`, uses that
time instead. An invalid `--at` is a usage error (exit 2). If the local time
can't be read: `shiver: warning: can't read the local time: ...` and no context
tag is active.

**Acceptance:**
- [x] Test `cli::say_at_uses_context` passes: with a pack of one `late` and one
      `morning` line, `--at 23:30` always gives the late line and `--at 07:00`
      the morning line.
- [x] Test `cli::say_at_rejects_bad_time` passes (exit 2).
- [x] `cargo deny check` passes with `jiff` added.

### R5: Docs

README: context tags, their windows, `--at`, and a note for pack authors.
`idle` stays in the backlog.

**Acceptance:**
- [x] Review: README updated.

## Out of scope

- Configurable windows or boost (spec 0010).
- `idle` detection, weekday tags.
