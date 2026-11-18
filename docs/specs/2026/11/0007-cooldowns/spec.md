---
id: 0007
name: cooldowns
status: done
created: 2026-11-18
---

# 0007: Cooldowns

Remember recent picks and avoid repeating lines, and the same voice twice in a
row. Human-facing overview: [onepager.html](onepager.html).

## Requirements

### R1: Line ids

Each loaded line has an id: the 64-bit FNV-1a hash of `voice`, a `\0` byte and
the line's (trimmed) text. `Line::id()` returns it. The id doesn't depend on
the pack, file or position of the line.

**Acceptance:**
- [x] Test `pack::tests::line_id_is_stable` passes: a fixed voice and text give
      a fixed, hard-coded id.
- [x] Test `pack::tests::line_id_depends_on_voice_and_text` passes.

### R2: Picking with history

`Chorus::pick_with_history(&filter, &history, &mut rng)` where `history` is a
slice of recent picks, oldest first, each with a line id and voice key.
Starting from the lines matching the filter:

1. **Fresh lines:** lines whose id isn't in the history. If there are any, the
   pool is the fresh lines.
2. **Fallback:** otherwise, sort the matching lines by when they were last
   shown (oldest first) and take the first half, rounded up. That's the pool.
3. **Voice turn:** if the filter has no voice, and the pool contains a line from
   a voice other than the last pick's voice, remove the last voice's lines from
   the pool.
4. Pick from the pool weighted by line weight.

`Chorus::pick` stays and behaves as `pick_with_history` with an empty history.

**Acceptance:**
- [x] Test `select::tests::prefers_unseen_lines` passes: with 6 lines, 6
      consecutive picks (feeding each pick into the history) are all different,
      for 20 seeds.
- [x] Test `select::tests::falls_back_to_oldest_half` passes: with every line
      in the history, picks only come from the older half.
- [x] Test `select::tests::voices_take_turns` passes: two voices with many
      lines never pick the same voice twice in a row over 50 picks.
- [x] Test `select::tests::voice_turn_allows_only_voice` passes: if only one
      voice has lines, it can repeat.
- [x] Test `select::tests::voice_filter_skips_voice_turn` passes.
- [x] Test `select::tests::history_keeps_weights` passes: with no history
      entries matching, weights still apply (10,000 picks, 1:9 split within
      85–95%).

### R3: History file

The binary stores the history at `<state dir>/shiver/history`
(`~/.local/state/shiver/history`; `dirs::state_dir()`), one pick per line:
`<unix seconds> <id as 16 hex digits> <voice key>`.

- Missing file: empty history.
- Unreadable file or a malformed line: `shiver: warning: ...`, and those lines
  are ignored.
- After `say` or `notify` picks a random line, the pick is appended and the file
  is rewritten with only the last 50 entries (written to a temporary file in the
  same folder, then renamed). If writing fails: warning, and the command still
  succeeds.
- One-off `notify --text` doesn't read or write the history.
- If no state folder can be found, cooldowns are skipped silently.

**Acceptance:**
- [x] Test `history::tests::round_trips` passes.
- [x] Test `history::tests::keeps_last_50` passes.
- [x] Test `history::tests::skips_malformed_lines` passes (with a warning count).
- [x] Test `cli::say_does_not_repeat_lines` passes: with a 3-line pack, 3
      `say` runs show 3 different lines.
- [x] Test `cli::history_uses_state_dir` passes (`XDG_STATE_HOME`).
- [x] Review: the one-off `notify --text` path doesn't read or write the
      history (a CLI test would send a real notification).

### R4: Tests stay isolated

CLI tests point `XDG_STATE_HOME` at a temporary folder, so running the tests
never touches the real history.

**Acceptance:**
- [x] Review: the `shiver()` test helper sets `XDG_STATE_HOME`.

### R5: Docs

The README explains cooldowns in a sentence or two and where the history lives.

**Acceptance:**
- [x] Review: README updated.

## Out of scope

- Configuring the history size, or turning cooldowns off.
- Time-based cooldowns.
- Commands to show or clear the history.
