# 0005: Rename Library to Chorus – tasks

- [x] 1. Move `library.rs` to `chorus.rs`, rename the type and module (R1)
- [x] 2. Rename variables, helpers and doc comments (R2)
- [x] 3. Same tests pass, CI checks clean (R3)

## Notes

- Done with `git mv` plus a word-boundary replace of `Library`, `library` and
  `lib` across `crates/**/*.rs`; no `lib.rs` mentions were in the code to catch.
- Test helpers that built a sample chorus are now `sample_chorus()`, so they
  aren't shadowed by `chorus` variables.
- 82 tests before and after; 17 of them are `chorus::tests::*`. Coverage 95.51%.
