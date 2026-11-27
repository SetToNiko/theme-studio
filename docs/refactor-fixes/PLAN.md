# Refactor Plan: Theme Studio, the new kit rules

<!--
The refactor-kit plan action wrote this file. You can edit it. The agents read it.
Keep the section numbers. The kit finds the sections by their numbers.
-->

## 0. Goal
Theme Studio obeys the new rules of the refactor kit. Success criteria (GOAL.md, baseline in SUCCESS_CRITERIA.md):
1. `file-limits.sh` counts lines of code (self-test in the gate). 2. Each `mod.rs` holds only declarations, each with `///` (gate). 3. The smallest visibility: `pub` only on the library API. 4. `studio/src/lib.rs` exists, and `main.rs` uses only the library. 5. No test file in `studio/src/`. 6. Integration tests in `studio/tests/` for each public module. 7. `# Examples` on each public item, `cargo test --doc` passes. 8. The API checklist has no open item. 9. The dev shell has nextest, mutants, fuzz, tokei, nightly (fuzz only), and the flake runs nextest and doc tests. 10. One fuzz target for each parser in section 3. 11. Coverage 82.74 % or more, `nix flake check` passes.
A pure refactor: the behavior does not change.
Goal locations:
- `mod.rs` with code: `keymap/mod.rs`, `fonts/mod.rs`, `config/mod.rs`, `settings_page/mod.rs`, `app/mod.rs`, `preview_term/mod.rs`, `ui/status/mod.rs`, `ui/mod.rs`, `ui/page_bodies/settings/mod.rs`, `system/download/mod.rs`, `app/tests/mod.rs`, `ui/overlays/mod.rs`. All 22 `mod.rs` files: declarations without `///`.
- Test files: `app/tests/{mod,input,layout,lifecycle,palette,pages_fonts,font_downloads,pages_settings}.rs`. Second test modules: `model/palette.rs:477`, `preview_term/pane.rs:409`.
- Library target: `studio/src/main.rs:5-13` (module list), `main.rs:50-162` (`main`, about 110 lines).
- Gates: `scripts/dev/file-limits.sh`, `flake.nix` (checks and dev shell), `studio/Cargo.toml` (lints).

## 1. Summary
The studio is one binary crate with 114 production files and about 30,900 lines with tests. The first refactor left clean modules with 0 dependency cycles, 0 clippy warnings, and no file over 500 lines. The new kit rules break in four places. Twelve `mod.rs` files hold code (`keymap/mod.rs`: 376 lines of code). The scenario tests of `app/` are in a separate folder. The crate has no library target, so 656 `pub` items have no API meaning. No parser has a fuzz target, and the dev shell lacks four tools and a nightly toolchain.

## 2. Architecture map (before the refactor)
<!-- The plan action writes this section one time. Do not change it later. The final action compares it with the code after the refactor (BEFORE_AFTER.md). -->

| Module | Purpose | Responsibilities | Depends on |
|---|---|---|---|
| `main.rs` | start the studio | arguments, `--init-config`, store migration, terminal setup, run loop start | all 9 modules |
| `app/` | editor state and actions | `App` (about 45 fields), keys, mouse, pop-ups, pages, saving, run loop | config, fonts, keymap, model, preview_term, settings_page, system |
| `ui/` | draw the screen | ratatui drawing of each page, pane, overlay, and status line, click targets in `app.hits` | app, keymap, model, fonts, settings_page, preview_term |
| `keymap/` | key bindings | contexts, actions, key notation, lookup, JSON load and save, presets | none |
| `config/` | config files | load, migrate, and save `keymaps.json` and `settings.json` | keymap |
| `settings_page/` | Settings page state | tabs, draft diff, rows, readme render, keymap presets | keymap, model |
| `fonts/` | Fonts tab | presets, installed fonts, tree, pick, download, preview, and tournament pages, sample render | model, system, keymap |
| `preview_term/` | live previews | nvim, shell, and yazi in PTYs, the draft palette file | model, system |
| `model/` | studio data and rules | colour, palette, theme, store, history, paths, tournament, samples | system |
| `system/` | outside world | desktop configs, downloads, prefetch threads, terminal output, switch scripts | model |

Cycles: none (the gate `cycles` gives 0).
Key numbers: 9 top-level modules and `main.rs`. 114 production files. 0 files over 500 lines of code. Largest file: `app/input/actions.rs`, 391 lines of code. 12 of 22 `mod.rs` files hold code. 656 `pub` items. 195 tests (4 ignored). Coverage 82.74 %.

## 3. Target architecture
- `studio/src/lib.rs`: only module declarations with `///`. The only public module is `cli`: `pub fn run()` and the argument types. The other modules are private or `pub(crate)`.
- `studio/src/main.rs`: calls `theme_studio::cli::run()`. No `mod` lines.
- Each `mod.rs`: a `//!` header and declarations with `///`. Its code moves into child modules with purpose names. A child module never has the name of its folder (clippy `module_inception`). Example: `keymap/map.rs`, not `keymap/keymap.rs`.
- Folders stay at 10 modules or fewer: `fonts/tab/` (the `FontsTab` parts), `fonts/pages/` (download, preview, tournament), and one group folder in `ui/` (the refactorer proposes the name).
- Tests: a `mod tests` at the end of each tested file. `app/test_support.rs` (`#[cfg(test)]`, no tests) holds the shared helpers `test_app`, `press`, `render`, `mouse`, `click_on`, `temp_store`. `studio/tests/cli.rs` tests the public API.
- Fuzz: `studio/fuzz/` (cargo-fuzz crate). `lib.rs` has `#[cfg(fuzzing)] pub mod fuzzing;`. `src/fuzzing.rs` has thin `pub fn` wrappers that call the `pub(crate)` parsers. `scripts/dev/fuzz-all.sh` runs each target.
- Parsers that get a fuzz target (criterion 10). A parser that reads a file gets a pure `&str` part first.
  - text: `Rgb::parse`, `Key::parse_seq`, `Action::from_id`, `Context::from_id`, `Underline::parse`, `settings_page::readme::render`
  - config JSON: `Keymap::from_json`, `parse_settings`, the keymaps-file parse
  - data JSON: `Palette::from_json`, `Theme::from_json`, `Preset::from_json`, the store and tournament JSON loads
  - downloads: `parse_fontsource`, `parse_nerd`, `sri`
  - desktop and system: `read_i3`, `read_rasi`, `read_bar`, `parse_i3blocks`, `scan_families` (fc-list text), the CLI arguments (`cli::parse_args`)
- Gates: `file-limits` (lines of code), `mod-layout`, `examples-doc`, `[lints.rust] unreachable_pub = "warn"` (criterion 3), and nextest with `cargo test --doc` in the package build.

Advanced techniques that the refactor adds:
- `#[cfg(fuzzing)]` module | `studio/src/fuzzing.rs` | the fuzz crate can call only `pub` items. Public parsers would add about 20 items to the API, each with `# Examples`.

## 4. Findings
| # | File(s) | Problem | Target violated | Impact |
|---|---|---|---|---|
| 1 | 12 `mod.rs` files (section 0) | code, `use` lines, and tests in `mod.rs` | mod-layout rule, T5 | H |
| 2 | all 22 `mod.rs` files | declarations without `///` | mod-layout rule | M |
| 3 | `app/tests/*.rs` | scenario tests in a separate folder | tests rule | H |
| 4 | crate | no library target, 656 `pub` items | criterion 3, C-STRUCT-PRIVATE | H |
| 5 | `main.rs:50-162` | `main` has about 110 lines, `--init-config` has no test | T3, criterion 6 | M |
| 6 | `ui/mod.rs:184`, `ui/page_bodies/settings/mod.rs:117` | two `truncate` functions with different overflow handling | T2 one source of truth | L |
| 7 | `ui/status/mod.rs:15` | a child imports `draw_tabs` and `truncate` from the parent `mod.rs` | T5 | L |
| 8 | `app/pages/settings/settings_actions.rs:352`, `settings_history_actions.rs:70` | the same file read and JSON parse, two copies | T2 | L |
| 9 | `fonts/presets.rs:311` | `load_active` returns an unnamed 4-tuple | T3 | L |
| 10 | `model/history.rs:28-35`, `system/switch.rs:80` | test hooks and `cfg!(test)` in production code | T4 testability | L |
| 11 | `ui/` (11 modules), `fonts/` (14 modules) after the moves | over the folder limit of 10 | limits | M |
| 12 | about 30 parsers (section 3) | no fuzz target | criterion 10 | M |
| 13 | `main.rs:64` | only the first argument is checked for `--help`, so `nosuch --help` gives the error for an unknown name. Test: `help_is_ignored_when_it_is_not_the_first_argument` | T3 (behavior, not fixed: pure refactor) | L |
| 14 | `main.rs:121` | an unknown option such as `--bogus` is taken as a name: "no palette named '--bogus'". The error does not say that a theme name is also valid. Test: `unknown_option_is_taken_as_a_name` | T7 (behavior, not fixed: pure refactor) | L |
| 15 | each `mod.rs` with intra-doc links in its `//!` header | a `///` on the parent declaration makes rustdoc resolve the `//!` links in the parent scope. Short links such as `[`Pane`]` then fail. | criterion 2, gate `doc` | M |

Findings 6 to 10 are in scope only when a step already changes the file (GOAL.md, scope).

## 5. Test coverage
- `app/`: partial. Tests only through `app/tests/` scenarios. No tests for `run.rs`, `persist.rs`, `state/nav.rs`, `state/messages.rs`.
- `ui/`: partial. Three inline test modules, the rest only through the app scenarios.
- `keymap/`, `config/`, `settings_page/`: good. No tests for `keymap/key.rs` (its tests are in `mod.rs`), `keymap/table.rs`, `config/settings.rs`, `settings_page/readme_layout.rs`.
- `fonts/`, `preview_term/`: partial. No tests for `filters.rs`, `pick.rs`, `download_page.rs`, `preview_page.rs`, `tournament_page.rs`.
- `model/`, `system/`: good. Each file has a test module.
- `main.rs`: partial. `studio/tests/command_line.rs` (P0.5, 17 tests) runs the binary for `-h`, `--help`, `--init-config`, and the errors before the terminal starts. No tests for the paths after the terminal starts, the timing log, and the history folder.

## 6. Plan

Step format: `- [ ] P<phase>.<n> <title>: <what changes> | Wave: <n> | Depends: <step IDs or none> | Owns: <paths> | Target: <criterion or target> | Check: <how to test>`
Each Rust step runs the checks for each change of the conventions, after P0.1. Each `Owns` path starts at the repository root. In the step text, a path without a folder is in `studio/src/`.

### Phase 0: Foundation and safety net
- [x] P0.1 Dev tools: add the flake input `rust-overlay`. Add `cargo-nextest`, `cargo-mutants`, `cargo-fuzz`, `tokei`, and a nightly cargo only for `cargo +nightly fuzz` to the dev shell. | Wave: 1 | Depends: none | Owns: flake.nix, flake.lock | Target: criterion 9. | Check: each tool runs in `nix develop`. `nix flake check` passes.
- [x] P0.2 Gate tests: the package build runs `cargo nextest run` (`useNextest`). The doc-test check moves to P1.3: a binary crate has no doc tests. | Wave: 2 | Depends: P0.1 | Owns: packaging/nix/package.nix | Target: criterion 9. | Check: `nix flake check` passes. nextest gives 191 passed, 4 skipped.
- [x] P0.3 Lines of code: `file-limits.sh` counts lines of code (no empty lines, no comment lines, no `/* */` lines) before the `#[cfg(test)]` module. It reads each character, so strings and char literals do not start a comment. Limit 500. A self-test runs in the gate. | Wave: 3 | Depends: P0.1 | Owns: scripts/dev/file-limits.sh, flake.nix | Target: criterion 1. | Check: the self-test passes. `nix flake check` passes.
- [x] P0.4 Gate `mod-layout`: a script fails on a `mod.rs` with a bad line. A bad line is not empty, not a comment, and not a declaration. A declaration without `///` above it is also bad. A ratchet: the count of bad files must not increase (baseline 22, `docs/refactor/lint-baseline.txt`). | Wave: 4 | Depends: P0.1 | Owns: scripts/dev/mod-layout.sh, scripts/dev/lint-counts.sh, docs/refactor/lint-baseline.txt, flake.nix | Target: criterion 2. | Check: the script gives 22 now. `nix flake check` passes. `lint-counts.sh` also reads the clippy line of the library.
- [x] P0.5 Safety net for `main.rs`: integration tests in `studio/tests/command_line.rs` run the binary (`CARGO_BIN_EXE_theme-studio`). They test `-h`, `--help`, `--init-config`, and each argument case that ends before the terminal starts. Each run gets a temporary HOME, `XDG_*`, and `THEME_STUDIO_*` through `Command::env`, with no `set_var`. Add the `.rs` files of `studio/tests/` to the source set of the package. | Wave: 5 | Depends: P0.2 | Owns: studio/tests/command_line.rs, packaging/nix/package.nix | Target: correctness. | Check: the tests pass on the unchanged code, also in `nix flake check`.
Completes: criterion 9. The coordinator measures the Phase 0 baselines of SUCCESS_CRITERIA.md (rows 1, 2, 7) after P0.4.

### Phase 1: Library target
- [x] P1.1 Library: add `lib.rs` with the module declarations of `main.rs`, each with `///`. Move the argument handling, `--init-config`, and the start into `cli.rs` (`pub fn run`). `main.rs` calls `theme_studio::cli::run()`. The tests of P0.5 do not change. The `fmt` gate formats `lib.rs` and `main.rs`. | Wave: 6 | Depends: P0.5 | Owns: studio/src/main.rs, studio/src/lib.rs, studio/src/cli.rs, flake.nix, studio/src/preview_term/mod.rs | Target: criteria 4, T3. | Check: `nix flake check` passes. The binary starts and `--help` gives the same text.
- [x] P1.2 Integration tests: `git mv studio/tests/command_line.rs studio/tests/cli.rs`. The `//!` header says that the tests cover the public API of `cli` through the binary that calls `cli::run`. Add a test of the type of `cli::run`. | Wave: 7 | Depends: P1.1 | Owns: studio/tests/cli.rs, studio/tests/command_line.rs | Target: criterion 6. | Check: `cargo nextest run` passes.
- [x] P1.3 Gate `examples-doc`: a script fails when a `pub` item that `lib.rs` exports has no `# Examples`. Add `# Examples` with `?` to each public item of `cli`. Add a flake check that runs `cargo test --doc`. | Wave: 7 | Depends: P1.1 | Owns: scripts/dev/examples-doc.sh, flake.nix, studio/src/cli.rs | Target: criterion 7, C-EXAMPLE. | Check: `cargo test --doc` passes. `nix flake check` passes.
Completes: criterion 4.

### Phase 2: `mod.rs` layout
Each step of this phase: move the code of each `mod.rs` in its folders into child modules with purpose names. Add `///` to each declaration. Move each test to the end of the file of its tested function. Change the `use` paths of the callers (no `pub use`). Report the new count of the `mod-layout` script: the coordinator lowers the baseline. Keep the cycle count at 0 and each folder at 10 modules or fewer.
- [ ] P2.1 `keymap/`: `context.rs`, `map.rs`, `lookup.rs`, `json.rs`, `edit.rs`. The key tests go to `key.rs`, the preset tests to `defaults.rs`. | Wave: 8 | Depends: P1.1 | Owns: studio/src/keymap/, studio/src/fonts/mod.rs, studio/src/ui/mod.rs, studio/src/ui/status/mod.rs, studio/src/ui/panes/preview.rs, studio/src/ui/overlays/help.rs, studio/src/ui/page_bodies/fonts_roles.rs, studio/src/ui/page_bodies/settings/mod.rs, studio/src/ui/page_bodies/settings/keymaps.rs, studio/src/settings_page/mod.rs, studio/src/settings_page/draft.rs, studio/src/settings_page/behaviour.rs, studio/src/settings_page/keymap_presets.rs, studio/src/config/mod.rs, studio/src/app/mod.rs, studio/src/app/input/focus.rs, studio/src/app/input/actions.rs, studio/src/app/state/nav.rs, studio/src/app/pages/settings/settings_keymaps_actions.rs, studio/src/app/pages/themes/presets_actions.rs, studio/src/app/tests/pages_settings.rs, studio/src/app/tests/font_downloads.rs | Target: criteria 2, 5. | Check: `nix flake check` passes. The test count does not decrease. Outside `studio/src/keymap/`, only access-path edits.
- [ ] P2.2 `model/` and `system/`: `///` on the declarations. `system/download/mod.rs` code to child modules (types, file helpers, channel). | Wave: 8 | Depends: P1.1 | Owns: studio/src/model/mod.rs, studio/src/model/settle.rs, studio/src/system/, studio/src/ui/page_bodies/fonts_download_preview.rs, studio/src/ui/page_bodies/fonts_download_details.rs, studio/src/fonts/preview_page.rs, studio/src/fonts/download_page.rs, studio/src/app/pages/fonts/tour_actions.rs, studio/src/app/pages/fonts/fonts_install_actions.rs, studio/src/app/pages/fonts/prefetch_actions.rs, studio/src/app/pages/fonts/fonts_preview_actions.rs, studio/src/app/pages/fonts/fonts_dl_actions.rs, studio/src/app/tests/pages_fonts.rs, studio/src/app/tests/pages_settings.rs, studio/src/app/tests/font_downloads.rs | Target: criterion 2. | Check: as P2.1. Outside `studio/src/model/` and `studio/src/system/`, only access-path edits.
- [ ] P2.3 `preview_term/`: `types.rs` (`PaneId`, `Mode`), `preview.rs`, `bar.rs`, `drafts.rs`. | Wave: 8 | Depends: P1.1 | Owns: studio/src/preview_term/mod.rs, studio/src/preview_term/types.rs, studio/src/preview_term/preview.rs, studio/src/preview_term/bar.rs, studio/src/preview_term/drafts.rs, studio/src/preview_term/pane.rs, studio/src/app/mod.rs, studio/src/app/input/focus.rs, studio/src/app/state/hit.rs, studio/src/app/tests/input.rs, studio/src/app/tests/layout.rs, studio/src/ui/desktop_preview.rs, studio/src/ui/pages.rs, studio/src/ui/panes/preview.rs | Target: criteria 2, 5. | Check: as P2.1. Outside the first five files, only access-path edits.
- [ ] P2.4 `config/`: `load.rs`, `files.rs`, `keymaps_file.rs`, `migrate.rs`, `settings_file.rs`. | Wave: 9 | Depends: P2.1 | Owns: studio/src/config/ | Target: criteria 2, 5. | Check: as P2.1.
- [ ] P2.5 `settings_page/`: `state.rs`, `diff.rs`, `rows.rs`. | Wave: 9 | Depends: P2.1 | Owns: studio/src/settings_page/ | Target: criteria 2, 5. | Check: as P2.1.
- [ ] P2.6 `fonts/`: `fonts/tab/` (tab, list, presets, keys) and `fonts/pages/` (download, preview, tournament). | Wave: 9 | Depends: P2.2, P2.3 | Owns: studio/src/fonts/ | Target: criteria 2, 5, folder limit. | Check: as P2.1.
- [ ] P2.7 `ui/`: `ui/mod.rs` code to `frame.rs` and `tabs.rs`, the helpers to `pane_chrome.rs` and `widgets.rs`. One group folder keeps `ui/` at 10 modules. `status/`, `overlays/`, `page_bodies/settings/` code to child modules. One `truncate` (finding 6). | Wave: 10 | Depends: P2.5, P2.6 | Owns: studio/src/ui/ | Target: criteria 2, 5, folder limit. | Check: as P2.1.
- [ ] P2.8 `app/`: `app/mod.rs` code to `app_state.rs`, `startup.rs`, `pointer.rs`, `queries.rs`, `sample.rs`, `editing.rs`, `clipboard.rs`. `Term` to `run.rs`. Move `setup_terminal` and `restore_terminal` from `cli.rs` to `run.rs`, next to `Term`. `run_in_terminal` stays in `cli.rs`, because it uses `ui::draw`. `open_matches` to `persist.rs`. `///` in each `app/` subfolder except `app/tests/`. | Wave: 10 | Depends: P2.4, P2.5, P2.6 | Owns: studio/src/app/, studio/src/cli.rs | Target: criterion 2, T5. | Check: as P2.1.
- [ ] P2.9 Doc gates: move the item scanner of `errors-doc.sh` and `examples-doc.sh` into one shared file `scripts/dev/lib/doc-items.sh`. The two gates source it. They stay two gates, each with one rule. The self-tests do not change. `flake.nix` gives each gate the folder `scripts/dev`, not one file. This step is not a `mod.rs` step. | Wave: 8 | Depends: P1.3 | Owns: scripts/dev/lib/doc-items.sh, scripts/dev/errors-doc.sh, scripts/dev/examples-doc.sh, flake.nix | Target: T2, T4 (commit `fadb431` fixed the same bug in the two copies). | Check: the two self-tests pass. `nix flake check` passes. `shellcheck` gives zero warnings.
Completes: none (criterion 2 completes in P3.9).

### Phase 3: Test locations
- [ ] P3.1 Test helpers: move the helpers of `app/tests/mod.rs` into `app/test_support.rs` (`#[cfg(test)] pub(crate) mod test_support;` in `app/mod.rs`). The test files use the new path. | Wave: 11 | Depends: P2.8 | Owns: studio/src/app/test_support.rs, studio/src/app/mod.rs, studio/src/app/tests/ | Target: criterion 5. | Check: the test count does not decrease.
- [ ] P3.2 Input tests: `app/tests/input.rs` to `app/input/actions.rs`, `app/input/mouse.rs`, `app/state/hit.rs`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/input.rs, studio/src/app/input/actions.rs, studio/src/app/input/mouse.rs, studio/src/app/state/hit.rs | Target: criterion 5. | Check: as P3.1.
- [ ] P3.3 Layout tests: `app/tests/layout.rs` to `app/state/layout.rs`, `app/state/messages.rs`, `app/state/focus.rs`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/layout.rs, studio/src/app/state/layout.rs, studio/src/app/state/messages.rs, studio/src/app/state/focus.rs | Target: criterion 5. | Check: as P3.1.
- [ ] P3.4 Palette tests: `app/tests/palette.rs` to `app/state/palette_editor.rs`, `app/input/colours.rs`, `app/input/overlay.rs`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/palette.rs, studio/src/app/state/palette_editor.rs, studio/src/app/input/colours.rs, studio/src/app/input/overlay.rs | Target: criterion 5. | Check: as P3.1.
- [ ] P3.5 Lifecycle tests: `app/tests/lifecycle.rs` to `app/pages/themes/`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/lifecycle.rs, studio/src/app/pages/themes/ | Target: criterion 5. | Check: as P3.1.
- [ ] P3.6 Settings tests: `app/tests/pages_settings.rs` to `app/pages/settings/`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/pages_settings.rs, studio/src/app/pages/settings/ | Target: criterion 5. | Check: as P3.1.
- [ ] P3.7 Font tests: `app/tests/pages_fonts.rs` and `app/tests/font_downloads.rs` to `app/pages/fonts/`. | Wave: 12 | Depends: P3.1 | Owns: studio/src/app/tests/pages_fonts.rs, studio/src/app/tests/font_downloads.rs, studio/src/app/pages/fonts/ | Target: criterion 5. | Check: as P3.1.
- [ ] P3.8 One test module for each file: merge `related_tests` into `tests` in `model/palette.rs`, and `input_tests` into `tests` in `preview_term/pane.rs`. | Wave: 12 | Depends: P2.2, P2.3 | Owns: studio/src/model/palette.rs, studio/src/preview_term/pane.rs | Target: criterion 5. | Check: as P3.1.
- [ ] P3.9 Close: remove `app/tests/`. The `mod-layout` gate becomes hard (0). A gate check fails on a `tests.rs` file or a test folder in `studio/src/`. | Wave: 13 | Depends: P3.2, P3.3, P3.4, P3.5, P3.6, P3.7 | Owns: studio/src/app/tests/, studio/src/app/mod.rs, scripts/dev/mod-layout.sh, docs/refactor/lint-baseline.txt | Target: criteria 2, 5. | Check: `nix flake check` passes. The test count is 195 or more.
Completes: criteria 2, 5.

### Phase 4: Visibility and the API guidelines
Each step of this phase: give each item the smallest visibility that works. Remove the dead code that the compiler then reports. Check the code of the folders against the API checklist, and fix each item or record a verified exception with its ID.
- [ ] P4.1 `app/`. | Wave: 14 | Depends: P3.9 | Owns: studio/src/app/ | Target: criteria 3, 8. | Check: `nix flake check` passes.
- [ ] P4.2 `ui/`. | Wave: 14 | Depends: P3.9 | Owns: studio/src/ui/ | Target: criteria 3, 8. | Check: as P4.1.
- [ ] P4.3 `keymap/`, `config/`, `settings_page/`. | Wave: 14 | Depends: P3.9 | Owns: studio/src/keymap/, studio/src/config/, studio/src/settings_page/ | Target: criteria 3, 8. | Check: as P4.1.
- [ ] P4.4 `fonts/`, `preview_term/` (finding 9). | Wave: 14 | Depends: P3.9 | Owns: studio/src/fonts/, studio/src/preview_term/ | Target: criteria 3, 8. | Check: as P4.1.
- [ ] P4.5 `model/`, `system/` (finding 10). | Wave: 14 | Depends: P3.9 | Owns: studio/src/model/, studio/src/system/ | Target: criteria 3, 8. | Check: as P4.1.
- [ ] P4.6 Gate: `[lints.rust] unreachable_pub = "warn"` in `Cargo.toml`, so clippy with `-D warnings` fails on a `pub` item outside the library API. | Wave: 15 | Depends: P4.1, P4.2, P4.3, P4.4, P4.5 | Owns: studio/Cargo.toml | Target: criterion 3. | Check: `nix flake check` passes.
Completes: criterion 3.

### Phase 5: Fuzz targets
Each step of this phase: give each parser in its list a pure `&str` or `&[u8]` part if it reads a file. Add a wrapper in `fuzzing.rs` and a target in `studio/fuzz/`. Run each new target for 60 seconds. Add each crash input as a unit test.
- [ ] P5.1 Fuzz crate: `studio/fuzz/` (cargo-fuzz), `#[cfg(fuzzing)] pub mod fuzzing;` in `lib.rs` with its comment, `fuzzing.rs`, `scripts/dev/fuzz-all.sh`, the first target `Rgb::parse`. The `//!` header of `lib.rs` gets a usage line and a `no_run` example (C-CRATE-DOC). | Wave: 16 | Depends: P4.6 | Owns: studio/fuzz/, studio/src/lib.rs, studio/src/fuzzing.rs, scripts/dev/fuzz-all.sh, studio/src/model/color.rs | Target: criterion 10. | Check: `nix develop .#fuzz --command cargo fuzz run rgb_parse -- -max_total_time=60` gives no crash. `nix flake check` passes.
- [ ] P5.2 Keymap and config parsers: `Key::parse_seq`, `Action::from_id`, `Context::from_id`, `Keymap::from_json`, `parse_settings`, the keymaps-file parse, `Underline::parse`, `readme::render`. | Wave: 17 | Depends: P5.1 | Owns: studio/fuzz/, studio/src/fuzzing.rs, studio/src/keymap/, studio/src/config/, studio/src/settings_page/, studio/src/app/state/hit.rs | Target: criterion 10. | Check: as P5.1, for each new target.
- [ ] P5.3 Model and font JSON: `Palette::from_json`, `Theme::from_json`, `Preset::from_json`, the store and tournament JSON loads. | Wave: 18 | Depends: P5.2 | Owns: studio/fuzz/, studio/src/fuzzing.rs, studio/src/model/, studio/src/fonts/presets.rs | Target: criterion 10. | Check: as P5.2.
- [ ] P5.4 System parsers and arguments: `parse_fontsource`, `parse_nerd`, `read_i3`, `read_rasi`, `read_bar`, `parse_i3blocks`, `scan_families`, `sri`, the CLI arguments. For the CLI arguments, add `pub(crate) fn parse_args(&[String])` to `cli.rs`. It returns a command enum and keeps the current behavior (findings 13 and 14 too). `run` calls it. Unit tests go at the end of `cli.rs`. Add no public API. | Wave: 19 | Depends: P5.3 | Owns: studio/fuzz/, studio/src/fuzzing.rs, studio/src/system/, studio/src/fonts/installed.rs, studio/src/cli.rs | Target: criterion 10. | Check: as P5.2. `scripts/dev/fuzz-all.sh 60` gives no crash.
Completes: criteria 10, 11 (the coordinator measures the coverage after P5.4).

## 7. Out of scope
- The 9 small fixes: the next refactor (HANDOFF.md, section 3, step 1).
- Terminal independence and note N1: HANDOFF.md step 2.
- The preview for each app: HANDOFF.md step 3.
- Other Linux distributions, and a security review: HANDOFF.md step 4.
- The 16 files with 300 to 400 lines of code: they are under the limit of 500.
- Findings 6 to 10 outside the files that a step changes.
- The move of the `--init-config` migration code (`init_config`, `make_first_theme`) from `cli.rs` into the modules that own the stores (Phase 1 review, note 3).

## 8. Outcome
<!-- The final action completes this section. -->

## 9. Architecture notes (pending)
<!-- The review action adds notes here. The revise action processes them. -->

## 10. Plan changelog
| Phase | Note | Decision | Reason |
|---|---|---|---|
| 0 | `cargo +nightly fuzz` needs rustup | The nightly toolchain and cargo-fuzz are in a separate dev shell `fuzz` | Nix has no rustup. The default shell keeps the rustc of nixpkgs. |
| 0 | `cargo test --doc` fails in a binary crate ("no library targets") | P0.2 does nextest only. P1.3 adds the doc-test check. P1.1 changes the `fmt` gate. P1.2 adds `studio/tests/*.rs` to the package source. | The checks need `lib.rs`. The package source set had only `studio/tests/fixtures`. |
| 0 | `tokei` misreads Rust char literals: 10 of 114 files differ | Criterion 1 uses the self-test of `file-limits.sh`. `tokei` is a cross-check only. | Approved by the user. |
| 0 | A unit test in `main.rs` cannot run `main()` with arguments | P0.5 writes integration tests that run the binary, and adds `studio/tests/*.rs` to the package source (from P1.2) | Cargo gives `CARGO_BIN_EXE_theme-studio` only to integration tests. The tests stay valid after P1.1. |
| 1 | P1.1 replied NEEDS for `preview_term/mod.rs`. Rustdoc merges the `///` of `mod preview_term;` in `lib.rs` with the `//!` header of the child module and resolves the links in the parent scope. The `doc` gate fails with 2 unresolved links. | Release `studio/src/preview_term/mod.rs` to P1.1. The 2 links get `crate::` paths. | Approved by the user. P2.3 and P4.4 own the file too, but they run in later waves. P1.1 stays one atomic commit. |
| 1 | P1.2 replied NEEDS for `studio/tests/command_line.rs`. `cli::run` reads `std::env::args()` and the environment. An in-process test gets the arguments of the test harness, and the rules forbid `set_var`. The tests of P0.5 already run the binary. | Release `studio/tests/command_line.rs` to P1.2. P1.2 renames it to `studio/tests/cli.rs` and adds a test of the type of `cli::run`. | Approved by the user. No other open step owns the file. A new file would copy the 17 tests of P0.5. |
| 1 | Move `setup_terminal` and `restore_terminal` from `cli.rs` next to `Term` | ACCEPT: P2.8 | The type, its setup, and its restore are in one file. `run_in_terminal` stays in `cli.rs`, because `app/` must not depend on `ui/`. |
| 1 | A pure argument parser or `run_with(args)` | ACCEPT, changed: `pub(crate) fn parse_args` in P5.4, no `run_with` | The fuzz target needs a pure function. Unit tests in `cli.rs` give the tests in the same process. No new public API. |
| 1 | Move the `--init-config` migration code into the modules that own the stores | DEFER: section 7 | It helps no success criterion. `cli.rs` has 207 lines, and each function is short. A `migrate_all` in another module adds one jump for the reader. |
| 1 | Merge `errors-doc.sh` and `examples-doc.sh` | ACCEPT, changed: P2.9, one shared scanner, two gates | Commit `fadb431` fixed the same bug in the two copies (T4). Two gates keep one rule for each gate. |
| 1 | Replace the regex scanner of the doc gates with rustdoc JSON | REJECT | It needs the nightly toolchain in the gates. Criterion 9 permits the nightly toolchain only for cargo-fuzz. |
| 1 | C-CRATE-DOC: the `//!` header of `lib.rs` has no example and no usage line | ACCEPT: P5.1 | P5.1 already changes `lib.rs`. Criterion 8 needs it. |
| 2 | P2.1 and P2.2 replied NEEDS. Without `pub use`, the moves break the `use` paths of 21 and 13 caller files outside `Owns`. `wave-merge.sh` skips a step that changes a file outside `Owns`. | Release the caller files to P2.1 and P2.2, for access-path edits only. The coordinator merges the `use` lines of `app/tests/pages_settings.rs` and `app/tests/font_downloads.rs` by hand (section 12). | Approved by the user. Each step stays one atomic commit. The later owners of these files (P2.4 to P2.8, P3) run in later waves. |
| 2 | P2.3 replied NEEDS. Without `pub use`, the move breaks the `use` paths of 8 caller files. The `//!` links of `preview_term/pane.rs` fail in the `doc` gate. | Release the 9 files to P2.3, for access-path edits only. P2.1 also changes 3 of these files. The coordinator merges their `use` lines by hand (section 12). | Approved by the user. P2.3 stays one atomic commit. |
| 2 | P2.2 changed one `//!` link in `model/settle.rs`, outside `Owns`. The new `///` in `model/mod.rs` makes the short link fail in the `doc` gate (finding 15). | Release `studio/src/model/settle.rs` to P2.2. | Approved by the user. No step in wave 8 owns the file. P2.2 stays one atomic commit. |

## 11. Progress
<!-- The kit updates this table after each action. -->
| Item | Status |
|---|---|
| Plan | done |
| Phase 0 safety net | done |
| Phase 1 steps | done |
| Phase 1 review | done |
| Phase 2 steps | open |
| Phase 2 review | open |
| Phase 3 steps | open |
| Phase 3 review | open |
| Phase 4 steps | open |
| Phase 4 review | open |
| Phase 5 steps | open |
| Phase 5 review | open |
| Documentation | open |
| Final review | open |

## 12. Shared files (parallel mode only)
| File | Steps that change it | Rule |
|---|---|---|
| `flake.nix` | P0.1, P0.4, P1.1, P1.3, P2.9 | each step is alone in its wave, or the only owner in its wave |
| `lib.rs` | P1.1, P5.1 | one owner in each wave |
| `packaging/nix/package.nix` | P0.2, P0.5 | one owner in each wave |
| `app/mod.rs` | P2.8, P3.1, P3.9 | one owner in each wave |
| `docs/refactor/lint-baseline.txt` | P0.4, each Phase 2 step, P3.9 | Phase 2 steps do not own it. Each step reports its new count. The coordinator writes the file after the merge of the wave. |
| the `use` lines of the callers | each Phase 2 step | a step changes only the `use` lines of files outside `Owns`. The coordinator merges conflicts in `use` lines by hand (`kit:reference/parallel-rules.md`, section 6). |
| `fuzzing.rs`, `studio/fuzz/Cargo.toml` | P5.1 to P5.4 | one step in each wave |
