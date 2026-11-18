# 0003: Continuous integration – tasks

- [x] 1. `xtask` crate, cargo alias, `check-specs` with tests for every rule (R2–R8)
- [x] 2. Workflow with seven jobs; actions pinned (R1)
- [x] 3. Measure coverage and set the minimum (R10)
- [x] 4. `deny.toml`; `cargo deny check` passes (R11)
- [x] 5. Doc build passes with warnings as errors (R12)
- [x] 6. Update `AGENTS.md` (R9)
- [x] 7. All checks pass locally
- [x] 8. User pushes; the seven checks pass on GitHub and coverage shows on the run page (R1, R10)

## Notes

- **Coverage minimum: 95% lines.** Measured 95.72% on 2026-11-18 (core 99–100%;
  binary lower because notifications and real sound playback can't run in tests).
- Actions pinned: `actions/checkout@v7.0.1`, `Swatinem/rust-cache@v2.9.3`,
  `taiki-e/install-action@v2.90.5`; `dtolnay/rust-toolchain` has no version
  tags, so it's pinned to a commit of its `stable` branch.
- The coverage step sets `shell: bash` so `pipefail` is on; GitHub's default
  shell would let `| tee` hide a coverage failure.
- `deny.toml`: graph limited to Linux targets (notify-rust pulls Windows crates
  otherwise). MPL-2.0 allowed for `option-ext` (via `dirs`). Duplicate crate
  versions only warn.
- Workspace `rust-version = "1.89"`: notify-rust and zbus require 1.89.
- The spec checker caught this spec's own missing `tasks.md` on its first run.
- 2026-11-18: run 44210944402 on d7b7b42 passed all seven jobs (checked via
  the GitHub API).
