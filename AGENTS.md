# AGENTS.md

Guidelines for coding agents (and humans) working on shiver.

shiver shows short lines attributed to named "voices" as desktop popups, with
optional sound. All voices, lines and sounds come from user-supplied packs.
The overall architecture is in [`docs/design.html`](docs/design.html).

## Spec-driven development

Every feature or non-trivial change starts as a spec. **No implementation
before the spec is approved.**

### Where specs live

```
docs/specs/<yyyy>/<mm>/<id>-<name>/
  onepager.html   # for humans: review and approve
  spec.md         # for agents: requirements and acceptance criteria
  tasks.md        # for agents: implementation steps and progress
```

- `<yyyy>/<mm>` is the year and month the spec was created.
- `<id>` is a 4-digit sequential number across all months: the highest existing
  id under `docs/specs/` plus one, starting at `0001`. Ids are never reused.
- `<name>` is a short kebab-case name, e.g. `pack-loading`.

Example: `docs/specs/2026/11/0001-pack-loading/`.

### Workflow

1. **Draft.** Write `onepager.html` and `spec.md` together, with status `draft`.
   Ask the user about anything unresolved before asking for approval. Open
   questions are settled in conversation, never left in the published files.
2. **Approve.** The user reviews `onepager.html`. When they approve, set the
   status to `approved` in both files.
3. **Plan.** Write `tasks.md`, then set the status to `in-progress`.
4. **Implement.** Work through `tasks.md` in order and tick off each step as it
   is finished, so another session can pick up where the last one stopped.
5. **Done.** The spec is done when every acceptance criterion in `spec.md` is
   checked off and `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
   and `cargo test` all pass. Then set the status to `done`.

If the plan has to change during implementation (a requirement is wrong,
missing or out of scope), stop. Update `spec.md` and `onepager.html`, explain the
change to the user and get approval again before continuing.

Status values: `draft`, `approved`, `in-progress`, `done`. The status in
`spec.md` is the source of truth; `onepager.html` mirrors it.

### `onepager.html`

A single page the user can review in a few minutes. Plain HTML, no scripts, no
external resources, linked to the shared stylesheet:

```html
<link rel="stylesheet" href="../../../../style.css">
```

Sections:

- **Title, id, date and status** at the top.
- **Problem:** what's missing or wrong, and why it matters.
- **Proposal:** the approach, in plain language. Diagrams are welcome; code
  only where a format or interface is the point of the spec.
- **Scope:** what's in, and explicitly what's out.
- **Decisions:** choices made and the reason for each.
- **Done when:** a short summary of the acceptance criteria.

### `spec.md`

Precise enough that an agent can implement it and verify it without guessing.

```markdown
---
id: 0001
name: pack-loading
status: draft
created: 2026-11-17
---

# 0001: Pack loading

One-paragraph summary. Human-facing overview: [onepager.html](onepager.html).

## Requirements

### R1: Load packs from the config directory

What must happen, including edge cases and error behaviour.

**Acceptance:**
- [ ] Test `shiver_core::pack::tests::loads_minimal_pack` passes.
- [ ] `shiver list` with no packs installed prints `no packs found` and exits 0.

### R2: ...

## Out of scope

- ...
```

- Number requirements `R1`, `R2`, ... and never renumber them; mark dropped
  ones as `(dropped)` instead of deleting them.
- Every acceptance criterion must be checkable: a named test, or a command with
  its expected output or exit code. "Works well" is not a criterion.

### `tasks.md`

```markdown
# 0001: Pack loading – tasks

- [ ] 1. Add `Pack` and `Voice` types with serde parsing (R1)
- [ ] 2. ...

## Notes

Anything learned during implementation that the next session should know.
```

- Steps are small and ordered; each names the requirements it covers.
- Tick a step only when its code, tests and checks are done.

## Repository layout

- `crates/shiver-core`: library with packs, voices and line selection. It does
  **no desktop I/O** (no notifications, no sound, no spawning processes), so it
  stays reusable by other front-ends.
- `crates/shiver`: the binary, with the CLI and outputs (stdout, notification,
  sound). Each output is a module implementing a shared output trait. Move an
  output into its own crate only when it brings heavy dependencies.
- `docs/design.html`: overall architecture. `docs/specs/`: specs (see above).
  `docs/style.css`: shared stylesheet for the HTML docs.

## Building and checking

Before saying any change is finished, run:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass. Don't silence clippy with `#[allow]` without a comment
saying why.

## Code style

- Errors: `thiserror` for typed errors in `shiver-core`, `anyhow` with
  `.context(...)` in the binary. Error messages should say what failed and on
  which file or value.
- No `unwrap()`/`expect()` outside tests, unless the invariant is obvious and
  stated in the `expect` message.
- Keep functions small and names plain. Prefer clear code over comments; comment
  the *why*, not the *what*.
- Public items in `shiver-core` get a short doc comment.

## Testing

- Unit-test pack parsing and line selection in `shiver-core`.
- Anything random takes an RNG parameter so tests can use a seeded RNG and
  assert exact results.
- Test pack fixtures live in the crate's `tests/` or a `testdata/` folder and
  must be original content.
- Outputs that touch the desktop (notifications, sound) are kept thin so the
  logic around them can be tested without a desktop session.

## Dependencies

- Keep the dependency tree small. Prefer well-maintained, widely used crates.
- Don't add dependencies that need system libraries at build time (e.g. ALSA
  headers via `rodio`/`cpal`, GTK) without discussing it first.
- Heavy or optional functionality goes behind a cargo feature or in its own crate.

## Content and licensing

- Code is `MIT OR Apache-2.0`.
- Example packs in the repo must be original text and sounds, licensed CC0 or
  CC BY 4.0, with the license stated in the pack folder.
- Never commit text, audio or images from games or other copyrighted works,
  including in tests and fixtures.

## Code review checklist

When reviewing a change, check:

- [ ] It matches an approved spec, and the spec's acceptance criteria are met.
- [ ] `shiver-core` still does no desktop I/O.
- [ ] New logic has tests; randomness is seeded in tests.
- [ ] `fmt`, `clippy` and `test` pass.
- [ ] Errors carry useful context; no stray `unwrap()`.
- [ ] No new dependency without a reason, and none needing system libraries
      unless agreed.
- [ ] No copyrighted content added.
- [ ] Docs (`design.html`, spec files, README) updated if behaviour changed.

## Git

- Commit only when asked.
- Commit messages: short imperative summary line, then a body if the why isn't
  obvious. Reference the spec id when relevant, e.g. `Load packs from config dir (0001)`.
