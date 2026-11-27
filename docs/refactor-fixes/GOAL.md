# Refactor Goal: Theme Studio, the new kit rules

<!-- The refactor-kit setup wrote this file. You can edit it. Keep it shorter than approximately 40 lines. -->

## Goal
After this refactor, Theme Studio obeys the new rules of the refactor kit. The next refactors (small fixes, terminal independence) then start from code that obeys the kit.

## Reason
The kit changed after the first refactor (kit commits `bca1c95`, `06de00d`, `8271127`). Each next refactor must obey the new rules, and agents must not fix the layout in each step. Now 12 of 22 `mod.rs` files hold code (`keymap/mod.rs`: 650 lines), and the crate has no library target.

## Success criteria
1. `scripts/dev/file-limits.sh` counts lines of code only. Empty lines and comment lines do not count. Its count agrees with `tokei` (Code column) for each file.
2. Each `mod.rs` holds only a `//!` header and module declarations, with a `///` comment on each declaration. A gate in `nix flake check` checks it.
3. Each module and item has the smallest visibility that works. `pub` is only on the API of the library target.
4. `studio/src/lib.rs` exists. `main.rs` uses only the library.
5. Unit tests are in a `#[cfg(test)] mod tests` at the end of the file of the tested function. No `tests.rs` file and no test folder are in `studio/src/`.
6. Integration tests are in `studio/tests/`. They use only the public API. Each public module has at least one integration test.
7. Each public item has `# Examples` (C-EXAMPLE). The examples use `?`, not `unwrap`. `cargo test --doc` passes.
8. The code obeys the checklist `.claude/skills/project-conventions/rust-api-guidelines.md`. The final review finds no open item. A deviation is a verified exception with the item ID.
9. The dev shell has cargo-nextest, cargo-mutants, cargo-fuzz, tokei, and a nightly toolchain only for cargo-fuzz. `nix flake check` runs `cargo nextest run` and `cargo test --doc`.
10. `studio/fuzz/` has one fuzz target for each function that parses outside input. The plan lists the functions. Each target runs 60 seconds with no crash.
11. Line coverage is not lower than the baseline. `nix flake check` passes.

## Scope
- In scope: `studio/src/`, `studio/tests/`, `studio/fuzz/`, the dev shell and the gates in `flake.nix` and `scripts/dev/`, and the documents that the new layout changes (`CONTRIBUTING.md`, `ARCHITECTURE.md`).
- Out of scope:
  - The 9 small fixes: the next refactor. HANDOFF.md, section 3, step 1, lists them. Commit `7f78141` has their goal.
  - Terminal independence and note N1: HANDOFF.md step 2.
  - The preview for each app: HANDOFF.md step 3.
  - Other Linux distributions, and a security review: HANDOFF.md step 4.

## New behavior and removals
No new features. No removals. A pure refactor.

## Constraints
- The project works after each step: `nix flake check` passes.
- No new runtime dependencies. You can add development tools to the dev shell.
- Never run `theme-switch`, `font-switch`, `themes-switch`, or `install.sh --uninstall` on the real desktop. Use the harnesses in `tests/` with a temporary `HOME` and `PREFIX`.

## Verification
1. `nix flake check` after each step and after each merge. Decide by the exit code.
2. The number of tests does not decrease. The number of ignored tests and of "skipped:" lines does not increase. `SUCCESS_CRITERIA.md` gives the baseline.
3. The script golden cases (`tests/scripts/run.sh`) do not change.
4. After Phase 0: the checks for each change (nextest, doc tests, llvm-cov, mutants, fuzz) on each step.
5. The reviewer checks that each changed file groups its code by purpose.
6. Manual check on the laptop after the last step: open the studio, show a font preview, and switch a theme.

## Contributor guide
`CONTRIBUTING.md`. This refactor applies all of its settings, except the settings that the new rules replace:
- the test locations (`app/tests/`): the new rule puts the tests in the file of the tested function, or in `studio/tests/`
- the file limit that counts all lines: the new rule counts lines of code only
