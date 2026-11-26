# Success Criteria: Theme Studio

<!--
Source: docs/refactor/GOAL.md. The refactor-kit setup measured the baselines again on 2026-11-26, at commit bd4df8f.
"Phase 0" in the Baseline column: the tool is not in the dev shell yet. The plan adds it, then measures the baseline. "manual": a person measures it.
The targets are proposals. Change them before the plan action if necessary.
At the end of the refactor, record the results. The final action uses them as evidence.
-->

## How to use this file

1. In Phase 0, measure each row that has "Phase 0" and record the value in the "Baseline" column.
2. The review action measures the rows again after each phase.
3. After each phase, run the measurements again. Record the values in the "Result" column.
4. Add the hard checks (greps, tests, lints) to `nix flake check`. Then the criteria stay true after the refactor.

Not all criteria have the same weight:
- Sections 1, 2, and 3 show if the refactor achieves its goal and keeps all functionality. They have the highest weight.
- Section 5 gives only an indication. You can improve these numbers without better code. Example: you divide code into many very small files. That makes the code more difficult for a junior developer, not easier.
- Section 6 applies only to the `[feature]` phase "Install without Nix". This phase starts after all refactor phases are complete.

---

## 1. Extensibility: the cost to add functionality

Main source: `docs/migration.md`, section "For the refactor: what still ties it to this setup". It says that the app integrations are hard-coded in the scripts.

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| To add a new app integration (for example, a terminal other than Ghostty), you change 2 files or fewer. You change no script logic. | On a temporary branch, add a sample integration. Run `git diff --stat`. List the modified files, not the added files. | manual (sample on a temporary branch). The integrations are hard-coded in `theme-switch.sh` and `font-switch.sh`. | 2 files or fewer, 0 logic files | 2 files (1 list entry, 1 generator), 0 logic files. P2.7, sample Alacritty colours. |
| One data list defines the app integrations: the files that each app reads, and the reload command for each app. | Count the locations that name each app. Example: `grep -ci ghostty scripts/theme-switch.sh scripts/font-switch.sh` | Number of mentions in the two scripts: ghostty 13, rofi 9, fontconfig 8, i3 6, gtk 5, pywalfox 3, i3blocks 2, nvim 1 | Each app has 1 entry in the list. Other mentions are only in comments. | share/integrations.json: 8 apps. The switch scripts name no app in code (comments only). |
| Only one location runs the reload commands for apps. | `grep -n -E 'pkill\|i3-msg' scripts/*.sh` | 4 lines in 2 scripts: pkill (theme-switch.sh:74, font-switch.sh:185) and i3-msg (theme-switch.sh:75, font-switch.sh:186) | 1 location | 1 location: reload_apps in scripts/lib/integrations.sh (P2.6). |
| Theme Studio has no wallpaper hook, login screen, or rofi picker (goal change). | `grep -rniE "wallpaper|ascii-boot|greeter|json_picker|rofi-picker" scripts packaging flake.nix` | 31 lines (wallpaper hook, login screen, rofi picker) | 0 | 0 after Phase R |
| To add a new palette, font preset, or theme, you add data. You change no code. | Add a sample to the data folder on a temporary branch. Run `git diff --stat`. | manual (sample on a temporary branch) | 0 code files | |

## 2. Change locality and structure

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| No code outside `studio/src/paths.rs` and `scripts/lib/paths.sh` contains a hard-coded system path. | `grep -n -E '/var/lib\|/etc/' scripts/*.sh studio/src/*.rs`. Ignore comments. | 4 (in `font-switch.sh`) | 0 outside the two paths files | |
| No reference to the old dotfiles location. | `grep -rn 'dotfiles-nixos' studio/src scripts share packaging` | 1 | 0 | |
| External programs start from fewer modules. Each such module has a name that tells its concern. | `grep -l 'Command::new' studio/src/**/*.rs` | 8 files: app, desktop, download, fonts, history, palette, paths, samples | Fewer than 8. `app.rs` and the UI modules start no programs. | |
| Modules are in folders by feature, not all in one folder. | Count the modules in each folder. | 35 modules in `studio/src/`, 0 subfolders | 10 modules or fewer in each folder | |
| No dependency cycles between modules. | `python3 scripts/dev/module-cycles.py` (counts all `crate::` paths in production code. `cargo modules --acyclic` sees only `use` statements: 13 modules) | 2 cycle groups, 21 of 35 modules: {font_tree, fonts, fonts_pick, keymap} and {app + 16 modules} | 0 groups | after Phase 4: 2 groups, 7 modules ({font_tree, fonts, fonts_pick, keymap}, {ui, settings_ui, fonts_ui}); the big group of 17 is gone. Ratchet in nix flake check. |
| Dependencies go in one direction: UI → application logic → data and system access. | Read the output of `cargo modules dependencies`. Make sure that no data module or system module imports a UI module. | 4 wrong-direction imports: theme → app, download → app (base64 only), prefetch → app, preview → ui | 0 imports in the wrong direction | after Phase 4: no feature module adds methods to App; app does not depend on ui (draw is passed in); nav and encoding are leaves. |


## 3. Functionality and testability

The constraint "All current functionality of the project must continue to work after the refactor" uses these criteria.

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| All checks run with one command. | `nix flake check` | Does not exist. The flake has no `checks` output. | Exists and passes | |
| The Rust unit tests pass. | `cd studio && nix develop .. --command cargo test` | 119 tests: 115 pass, 4 ignored | All pass. 115 or more pass. The number of ignored tests does not increase. | |
| The tests that skip do not increase. Fixtures let the data tests run in the Nix sandbox. | `cargo test -- --nocapture 2>&1 \| grep -c '^skipped:'` | 15 locations in the code can print "skipped:". Number at run time: 1. Nix sandbox: 13 before P0.5, 10 after (fixtures) | 0 in the Nix sandbox | |
| The package builds. | `nix build` | Passes | Passes after each step | |
| The switch scripts have golden-file tests. | Count the scripts that have a golden-file test (refer to docs/refactor/GOAL.md, Verification, item 3). | 0 of 3 | 3 of 3. (The login-screen part is removed in Phase R.) | 3 of 3 after Phase 0 (14 cases). The login-screen part of font-switch is removed in R.2. |
| The characterization tests and golden files pass without changes from the start to the end. | No commit changes the expected values. Examine `git log -p` on the test files and the golden files. | — | 0 changes, except in `[feature]` steps | |
| The manual checklist passes after each phase that changes the scripts or the studio. | `docs/migration.md`, section "Checklist (after each step, as it applies)", items 1 to 9. Also docs/refactor/GOAL.md, Verification, item 6. | manual, before Phase 1 | All items pass | |
| Coverage of the changed modules does not decrease. | `cargo llvm-cov --summary-only` (in the dev shell) | lines 82.72% (3,099 of 17,930 missed), regions 82.83% | Equal to or more than the baseline | |

## 4. Audience test (A1 Beginner)

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| Each module has a comment at the top that gives its purpose. | `grep -L '^//!' studio/src/**/*.rs`. For the scripts: the comment block at the top of each file. | 35 of 35 Rust files | All files, also the new files | |
| ARCHITECTURE.md describes each module, the dependency direction, and how to add an app integration. | The file exists. Each module in the code has an entry. | Does not exist | Exists and is complete | |
| From the entry point to the result of one feature, a reader needs 3 jumps or fewer. | Follow these features manually: switch a theme, apply a font, save a palette. Count the jumps. | manual | 3 or fewer | |
| The "How to extend" walk-through in ARCHITECTURE.md works. | A person or a new agent follows the walk-through and adds a sample app integration without other help. | — | Works | |
| Each new behavior has documentation. | Each `[feature]` step in docs/refactor/PLAN.md has a section in `docs/theme-studio.md` or ARCHITECTURE.md. | — | All `[feature]` steps | |

## 5. Size and complexity (indication only)

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| File length. | `wc -l studio/src/**/*.rs scripts/*.sh` | 27 of 35 Rust files have more than 300 lines. 6 files have more than 1000 lines. Largest: `app.rs` 4948, `ui.rs` 2190, `fonts.rs` 1791. | No file has more than approximately 1000 lines. Most files have 300 lines or fewer. | |
| Function length. Guideline 30 lines, longer only with a verified reason (PRINCIPLES.md, section 3). | `lizard -l rust -l bash studio/src scripts` | 191 of 809 functions have more than 30 lines | Less than the baseline | |
| Cyclomatic complexity for each function. | `lizard -l rust -l bash -C 10 studio/src scripts` | 104 functions over 10 | 10 or less | |
| Clippy warnings, default lints. | `cargo clippy --all-targets` | 1 | 0 | |
| Clippy warnings, pedantic + style + correctness (GOAL.md, criterion 7). Ratchet: the count never increases. | `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::style -W clippy::correctness` | 490 (42 lints. Largest: cast lints about 215, many_single_char_names 53, semicolon_if_nothing_returned 41) | 0. Each allowed lint uses `#[expect(..., reason = "...")]`. | |
| Rustdoc (GOAL.md, criterion 8). | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`. Count `# Errors` sections for functions that return `Result`. | Fails: 3 "unclosed HTML tag" errors. 0 `# Errors` sections for 51 functions that return `Result`. | Passes. Each function that returns `Result` has `# Errors`. | |
| Shellcheck warnings. | `shellcheck scripts/*.sh scripts/lib/*.sh` | 14 findings | 0 | 4 after Phase R |
| Duplicate code. | `lizard -l rust -l bash -Eduplicate studio/src scripts` (jscpd is not in nixpkgs) | 1.13% | Less than the baseline | |

## 6. Feature: install without Nix

This phase starts after all refactor phases are complete. Source: `docs/migration.md`, section "For the refactor: what still ties it to this setup", items "Install without Nix" and "the studio's tools".

Support for other distros is not part of this refactor. A later project adds it. Thus, these criteria do not test specific distros.

| Criterion | How to measure | Baseline | Target | Result |
|---|---|---|---|---|
| One command installs Theme Studio without Nix: the studio, the commands `theme-switch`, `themes-switch`, and `font-switch`, the share files, and a desktop entry. | Run the install command with a temporary install prefix (for example `PREFIX=$(mktemp -d)`). Add `$PREFIX/bin` to PATH. Make sure that each command is on PATH: `command -v theme-studio theme-switch themes-switch font-switch`. | Does not exist | Exists | |
| The installed commands find the share files without Nix. | Run the script golden-file tests against the commands in the temporary prefix, not against the checkout. Do not set THEME_STUDIO_SHARE. | — | All pass | |
| The README lists all runtime dependencies. Examples: jq, rofi, fontconfig, ImageMagick, nvim, yazi, git, curl, tar, xz. | Compare the list in the README with the programs that the scripts and the studio start (`grep` for the command names). | Not listed | Each program is listed | |
| The README describes the install without Nix. | The README has a section with the install command and the dependencies. | No section | Section exists | |
| The Nix install continues to work. | `nix build` and `nix flake check` | — | Pass | |
