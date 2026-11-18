---
id: 0003
name: ci
status: in-progress
created: 2026-11-18
---

# 0003: Continuous integration

Run formatting, clippy, tests, coverage, dependency checks, a doc build and a
spec-format check on GitHub Actions for every push to `main` and every pull
request. Human-facing overview:
[onepager.html](onepager.html).

## Requirements

### R1: Workflow

`.github/workflows/ci.yml` runs on `push` to `main` and on `pull_request`, on
`ubuntu-latest` with stable Rust. Seven jobs, each named after its check:

| Job | Command |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `test` | `cargo test --workspace` |
| `coverage` | `cargo llvm-cov` (R10) |
| `deny` | `cargo deny check` (R11) |
| `doc` | `cargo doc` with warnings as errors (R12) |
| `specs` | `cargo xtask check-specs` |

Third-party actions are pinned to exact versions. Cargo builds are cached.

**Acceptance:**
- [x] The workflow file exists with the seven jobs above.
- [ ] Manual: after pushing, the seven checks pass on GitHub.

### R2: `xtask` crate

An `xtask` crate at `xtask/` (workspace member, `publish = false`) with a
`check-specs` command, and a cargo alias in `.cargo/config.toml` so
`cargo xtask check-specs` works. It checks `docs/specs/` relative to the
workspace root, prints one line per problem as `<path>: <problem>`, and exits 1
if there are any problems, else prints `specs ok (<n> specs)` and exits 0.

**Acceptance:**
- [x] `cargo xtask check-specs` passes on specs 0001, 0002 and 0003.
- [x] Test `tests::reports_all_problems_not_just_first` passes.

### R3: Folder layout

Every folder at `docs/specs/<yyyy>/<mm>/` is a spec folder named
`<id>-<name>`:

- `<yyyy>` is 4 digits, `<mm>` is `01`–`12`.
- `<id>` is 4 digits; `<name>` is lowercase kebab-case (`a-z`, `0-9`, `-`).
- Ids are unique across all specs.
- Files directly in `docs/specs/` (other than these folders) are not allowed.

**Acceptance:**
- [x] Test `tests::rejects_bad_year_or_month` passes.
- [x] Test `tests::rejects_bad_folder_name` passes.
- [x] Test `tests::rejects_duplicate_ids` passes.

### R4: Required files

- `spec.md` and `onepager.html` always.
- `tasks.md` when the status is `in-progress` or `done`.

**Acceptance:**
- [x] Test `tests::requires_spec_and_onepager` passes.
- [x] Test `tests::requires_tasks_once_started` passes.

### R5: `spec.md` front matter

`spec.md` starts with a `---` block containing `id`, `name`, `status` and
`created`:

- `id` and `name` equal the folder's.
- `status` is one of `draft`, `approved`, `in-progress`, `done`.
- `created` is a valid `yyyy-mm-dd` date whose year and month match the folder.

**Acceptance:**
- [x] Test `tests::requires_front_matter_fields` passes.
- [x] Test `tests::front_matter_matches_folder` passes.
- [x] Test `tests::rejects_unknown_status` passes.
- [x] Test `tests::rejects_bad_created_date` passes.

### R6: Requirements have acceptance criteria

Each `### R<n>: ...` heading in `spec.md` is followed, before the next `##` or
`###` heading, by an `**Acceptance:**` line and at least one `- [ ]` or `- [x]`
item.

**Acceptance:**
- [x] Test `tests::requirement_needs_acceptance` passes.

### R7: One-pager

`onepager.html` links `../../../../style.css` and contains
`<span class="status">STATUS</span>` with the same status as `spec.md`.

**Acceptance:**
- [x] Test `tests::onepager_links_stylesheet` passes.
- [x] Test `tests::onepager_status_matches_spec` passes.

### R8: Done means done

When the status is `done`, `spec.md` and `tasks.md` contain no `- [ ]` items.

**Acceptance:**
- [x] Test `tests::done_spec_has_no_open_boxes` passes.

### R9: Documented

`AGENTS.md` lists `cargo xtask check-specs`, coverage and `cargo deny check`
with the other checks to run before a change is finished, and mentions the CI
workflow.

**Acceptance:**
- [x] Review: `AGENTS.md` updated.

### R10: Coverage

The `coverage` job runs `cargo llvm-cov` over the workspace, excluding `xtask`,
and fails if line coverage is below a minimum set in the workflow. The minimum
is the coverage measured when this spec is implemented, rounded down to a whole
percent, so coverage can't drop. The summary table is added to the GitHub run
summary, also when the job fails.

**Acceptance:**
- [x] `cargo llvm-cov --workspace --exclude xtask --fail-under-lines <min>` passes locally.
- [x] The minimum is written in the workflow and in `tasks.md`.
- [ ] Manual: the coverage table shows on the GitHub run summary page.

### R11: Dependency licenses and advisories

`deny.toml` makes `cargo deny check` fail on:

- a dependency whose license isn't on an allow list of permissive licenses
  compatible with `MIT OR Apache-2.0`;
- a dependency with a known security advisory, or one that is yanked;
- a dependency from anywhere other than crates.io.

**Acceptance:**
- [x] `cargo deny check` passes locally.

### R12: Docs build

`cargo doc --workspace --no-deps --document-private-items` with
`RUSTDOCFLAGS=-D warnings` passes, so broken doc links fail CI.

**Acceptance:**
- [x] The doc command passes locally.

## Out of scope

- Release builds, publishing, other operating systems or toolchains.
- Hosted coverage services (Codecov etc.) and spell-checking.
- Checking that test names in acceptance criteria exist.
- Branch protection (configured on GitHub by the user).
