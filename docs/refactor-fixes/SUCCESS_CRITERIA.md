# Success Criteria: Theme Studio, the new kit rules

<!-- The refactor-kit setup wrote this file. The plan action copies these rows into section 0 of PLAN.md. Baseline measured on 2026-11-26, on the branch refactor/kit-rules, commit 7f78141. "Phase 0" means that the tool is not available yet. "manual" means that a person or the final review measures it. -->

| # | Criterion | Measure command | Baseline | Target |
|---|---|---|---|---|
| 1 | `file-limits.sh` counts lines of code | `scripts/dev/file-limits.sh --selftest`, and `--list` against `tokei` as a cross-check | The gate counted all lines (example: `app/input/actions.rs`, 474 lines, 391 lines of code). Phase 0 (P0.3): the self-test passes. The count agrees with `tokei` on 104 of 114 files. The 10 other files have char literals such as `'"'`, which `tokei` misreads. | the self-test passes in the gate |
| 2 | Each `mod.rs` holds only declarations, each with `///` | The gate `mod-layout` (Phase 0). Now: a `grep` for lines that are not empty, not a comment, and not a `mod` declaration. | 12 of 22 files have other lines. 22 of 22 files have a declaration without `///`. | 0 of 22 |
| 3 | The smallest visibility | `grep -rE '^\s*pub (fn\|struct\|enum\|trait\|const\|static\|type\|mod) ' studio/src \| wc -l` | 656 `pub` items. No library API. | Only the library API is `pub`. The plan sets the number. |
| 4 | Library target | `test -f studio/src/lib.rs` | no | yes. `main.rs` uses only the library. |
| 5 | No test files in `studio/src/` | `find studio/src -name tests.rs -o -path '*/tests/*.rs' \| wc -l` | 8 (`app/tests/`) | 0 |
| 6 | Integration tests | `find studio/tests -maxdepth 1 -name '*.rs' \| wc -l` | 0 | 1 or more for each public module |
| 7 | `# Examples` on each public item | A gate script (Phase 0). Now: `grep -rc '/// # Examples' studio/src` | 1 section in the crate. The count of public items without `# Examples`: Phase 0. | 0 public items without `# Examples`. `cargo test --doc` passes. |
| 8 | Rust API Guidelines checklist | The final review, against `.claude/skills/project-conventions/rust-api-guidelines.md` | manual | 0 open items. Each deviation is a verified exception with its item ID. |
| 9 | Tools in the dev shell. `nix flake check` runs nextest. | `command -v <tool>` in `nix develop`. `grep -c nextest flake.nix` | 5 tools are missing: cargo-nextest, cargo-mutants, cargo-fuzz, a nightly toolchain, tokei. `flake.nix` has no nextest. | 0 tools missing. The flake runs `cargo nextest run` and `cargo test --doc`. |
| 10 | Fuzz targets | `ls studio/fuzz/fuzz_targets` | No `studio/fuzz/` | 1 target for each parser of outside input in the plan list. Each target runs 60 seconds with no crash. |
| 11 | Line coverage | `cd studio && cargo llvm-cov --summary-only` (the TOTAL row, the Lines column) | 82.74 % (3122 of 18083 lines without a test) | 82.74 % or more |
| 12 | Tests | `cd studio && cargo test -- --nocapture`: the "test result" line, and the count of "skipped:" lines | 191 passed, 4 ignored (195 tests). 1 "skipped:" line. | 195 tests or more. 4 ignored or fewer. 1 "skipped:" line or fewer. |
| 13 | Gates | `nix flake check` (the exit code) | Not measured in this setup. HANDOFF.md says that it passes on `main`. | exit code 0 |
