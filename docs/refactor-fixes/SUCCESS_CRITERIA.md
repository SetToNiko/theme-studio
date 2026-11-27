# Success Criteria: Theme Studio, small fixes

<!--
The refactor-kit setup (part 6) wrote this file on 2026-11-26, from main at c229328.
The review action measures again after each phase. The final action records the results.
Run each command from the project root, in `nix develop`.
-->

## How to use this file

1. The "Baseline" column has the value that the setup measured. "Phase 0" means: the plan writes the test or the check in Phase 0, and that test fails on the current code.
2. After each phase, the kit runs the measure commands again and records the results in the "Result" column.
3. The criterion numbers are the numbers of `GOAL.md`.

## 1. The fixes

| # | Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|---|
| 1 | No production code in `settings_page` uses `crate::app` | `for f in studio/src/settings_page/*.rs; do awk '/#\[cfg\(test\)\]/{exit} /crate::app/{print FILENAME": "$0}' "$f"; done` (count the lines) | 3 | 0 | |
| 2 | `themes-switch` with a missing palette changes no file | `find tests/scripts/golden/themes-switch-missing-palette/home -type f` (count the files) and `cat tests/scripts/golden/themes-switch-missing-palette/exit` | 8 files, exit 1 | 0 files, exit 1 | |
| 3a | The previews are in the cache folder | `grep -n 'previews' studio/src/model/paths.rs` shows `XDG_CACHE_HOME` | `XDG_DATA_HOME` | `XDG_CACHE_HOME` | |
| 3b | The studio moves an old previews folder one time | a Rust test with a temporary data folder and a temporary cache folder | Phase 0 | the test passes | |
| 3c | No install and no uninstall touches the previews folder | a step of `tests/install/run.sh` that puts a file in the previews folder, then installs and uninstalls | Phase 0 | the file stays | |
| 4 | `real_configs_parse` reads the desktop folder | `grep -n -A2 'fn real_configs_parse' studio/src/system/desktop.rs` | reads `Store::locate().dir` | reads `store.desktop` | |
| 5a | One XDG rule: no direct read of an XDG base variable outside the one helper | `grep -nE 'var_os\("XDG_(CONFIG\|DATA\|STATE\|CACHE)_HOME' studio/src/model/paths.rs` (count the lines) | 3 (lines 47, 60, 117) | 0 | |
| 5b | Each public path function has a test | for each `pub fn` before `#[cfg(test)]` in `paths.rs`, search its name in the test module | 12 of 19 without a test | 0 without a test | |
| 5c | A test compares `paths.sh` with `paths.rs` | a test that runs `scripts/lib/paths.sh` and compares each shared path | 0 tests | 1 test, it passes | |
| 6a | `draft_changes` returns 0 when only other keys differ | `grep -n 'n.max(1)' studio/src/app/pages/settings/draft_actions.rs`, and a Rust test | `n.max(1)` present, no test | no `n.max(1)`, the test passes | |
| 6b | `Confirm` on a Gap row does not move the cursor | an `App` test in `app/tests/` | Phase 0 | the test passes | |
| 7 | The old documents are in the archive | `ls docs/plan.md docs/migration.md docs/archive/` | in `docs/`, no `docs/archive/` | only in `docs/archive/`, with a note | |
| 8 | `--init-config` has a test | `grep -rn 'init.config' studio/src --include=*.rs` shows a `#[test]` | 0 tests | 1 test or more | |
| 9 | `CONTRIBUTING.md` section 7 does not list the fixed items | `awk '/^## 7/{f=1} f' CONTRIBUTING.md \| grep -cE 'settings_page uses app\|themes-switch can half\|previews folder\|real_configs_parse\|n\.max\|--init-config\|docs/plan.md\|XDG differences\|compares .paths.sh.\|Gap rows'` | 10 | 0 | |

## 2. Gates and tests

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| All gates pass | `nix flake check`, exit code | 0 | 0 | |
| Rust tests | `#[test]` and `#[ignore` in `studio/src` (`grep -rn`, count) | 195 tests, 4 ignored | more tests, 4 ignored or fewer | |
| "skipped:" lines | `grep -rn 'eprintln!("skipped' studio/src` (count) | 14 | 14 or fewer | |
| Script golden cases | `grep -cE '^\s*run_case [a-z]' tests/scripts/run.sh` (a second check: `ls tests/scripts/golden \| wc -l`) | 22 | 22 or more | |
| Lint baseline | `scripts/dev/lint-counts.sh` | clippy 0, shellcheck 0, cycles 0 | all 0 | |

## 3. Manual check

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| The studio works on the laptop | After the last step: open the studio, show a font preview, switch a theme | manual | it works as before | |
