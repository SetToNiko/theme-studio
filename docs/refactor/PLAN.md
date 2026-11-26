# Refactor Plan: Theme Studio

<!--
The refactor-kit plan action wrote this file. You can edit it. The agents read it.
Keep the section numbers. The kit finds the sections by their numbers.
-->

## 0. Goal
A junior developer can read the codebase, quickly find the purpose of each module, and understand the architecture. The code is easy to maintain and extend.
Criteria: (1) new app integration = 2 files or fewer, no script logic (2) one data list defines the integrations (3) system paths only in `paths.rs` / `paths.sh` (4) `nix flake check` + manual checklist pass (5) purpose comment in each file, ARCHITECTURE.md lists each module (6) [feature] one command installs without Nix (7) clippy pedantic ratchet to 0 (8) rustdoc check passes, `# Errors` / `# Panics` documented.
Goal locations:
- Integrations: `scripts/theme-switch.sh:36-76` (i3, colors.sh, current, Ghostty, rofi, Pywalfox, reloads), `scripts/font-switch.sh:86-187` (Ghostty, i3, rofi, lock screen, login screen, GTK, fontconfig, reloads), `scripts/themes-switch.sh:45-49`
- Reloads: `theme-switch.sh:74-77`, `font-switch.sh:185-186`
- System paths: `font-switch.sh:102-103`, `studio/src/download.rs:544`
- User paths outside paths: `store.rs:181`, `fonts.rs:291,300,757`, `theme.rs:160,169`, `config.rs:24`, `history.rs:34`; `home()` in 5 modules
- Old dotfiles reference: `studio/src/samples/code/go.go:49`

## 1. Summary
Theme Studio has a 25k-line Rust TUI (35 modules in one flat folder) and 4 bash switch scripts. The app integrations are hard-coded in two scripts: about 8 apps, each with its own output and reload command. `App` (`app.rs`, about 2,450 production lines, 60 public fields) is a god object: nearly every module depends on it, and it depends on them, so the module graph is full of cycles. 12 files are over the 500-line limit, `home()` exists 5 times, and user paths are spread over 4 modules and 3 scripts. The studio applies themes only through the switch scripts (`paths::switch_command`), so the integration logic exists only in bash.

## 2. Architecture map (before the refactor)
<!-- The plan action writes this section one time. Do not change it later. The final action compares it with the code after the refactor (BEFORE_AFTER.md). -->

| Module | Purpose | Responsibilities | Depends on |
|---|---|---|---|
| `studio/src/main.rs` | Entry point | terminal setup, event loop start, module list | app, store |
| `app.rs` | Editor state and input handling | 60-field App state, palette editing + undo, pane layout + resize, overlay dialogs, key dispatch, mouse, fonts tab, themes, messages | 23 modules |
| `ui.rs` | Drawing | all pages, panes, overlays, status line | app + 13 modules |
| `keymap.rs` | Keymaps | action names, contexts, bindings, keymap files | fonts |
| `fonts.rs` | Fonts column of the Main page | font presets, installed fonts (fc-list), sample rendering jobs, DPI (xrdb) | color, fonts_pick, font_tree, keymap, palette, paths, samples, store |
| `settings.rs` | Settings page (F3) | Keymaps, Behaviour, History tabs, draft handling | app, color, config, docs, download, draft, history, hit, keymap, store |
| `settings_ui.rs` | Drawing the Settings page | tab strip, keymaps, behaviour, history views | app, behaviour, color, docs, hit, keymap, paths, presets, settings, ui |
| `download.rs` | Downloading fonts | Fontsource and Nerd Fonts lists, downloads, preview fontconfig | app (base64), store |
| `tour_page.rs` | Tournament tab | pool choice, preview fetch, rounds | app, download, fonts, fonts_dl, keymap, prefetch, samples, settings, settle, tournament |
| `config.rs` | The studio's own settings | settings.json, sizes, load and save | fonts, keymap, store |
| `behaviour.rs` | Behaviour tab | settings values in groups, editing | app, config, hit, keymap, palette, prefetch, settings |
| `fonts_dl.rs` | Download fonts page (F2) | tabs, search, install actions | app, download, fonts_pick, fonts_preview, hit, keymap, prefetch, settings |
| `samples.rs` | Preview texts for fonts | sample texts, ImageMagick rendering | palette |
| `preview.rs` | Live preview | model i3 desktop, real programs (nvim) in a PTY | color, desktop, palette, paths, store, ui |
| `fonts_dl_ui.rs` | Drawing the Download page | tabs, chips, lists | app, download, fonts_dl, fonts_preview, hit, samples, settle, tour_ui |
| `palette.rs` | A palette | 34 keys, base-colour rules, jq reading | color, paths, store |
| `history.rs` | Version history of config files | snapshots, restore, dates | none |
| `hit.rs` | Clickable areas | hit map, borders, hover | app (Page), preview |
| `tournament.rs` | Font tournament | pairing, results | history, store |
| `fonts_ui.rs` | Drawing the fonts column | roles, installed list | app, fonts, hit, keymap, settle, ui |
| `fonts_preview.rs` | Fonts page preview | draw a font before install, menus | app, download, fonts, fonts_dl, keymap, samples, settle |
| `docs.rs` | README tab | find and render docs/theme-studio.md | paths, store |
| `presets.rs` | Keymap preset browser | preset list, apply | app, hit, keymap, settings |
| `desktop.rs` | The preview's desktop model | read real i3, rofi, i3blocks configs, run bar blocks | color, palette, store |
| `tour_ui.rs` | Drawing the Tournament tab | setup, rounds, results | app, hit, settle, tour_page |
| `prefetch.rs` | Fetch font previews early | background queue | app, download, settings |
| `store.rs` | Files the studio reads and writes | palettes.json, current palette, JSON writes, own `home()` | color, history, palette, paths |
| `font_tree.rs` | Font list tree | group families into two levels | fonts |
| `fonts_pick.rs` | Fonts for roles | kind of font for each role | fonts |
| `paths.rs` | Where things are | data, share, scripts, desktop configs, switch command | store (tests only) |
| `draft.rs` | Unsaved Settings changes | draft keymaps and behaviour | app, config, keymap, settings, store |
| `color.rs` | Colour type | hex, HSV, HSL, contrast | none |
| `settle.rs` | Image size settling | wait until a pane size is stable | none |
| `out.rs` | Terminal output | frame buffering | none |
| `theme.rs` | Themes | theme data, themes.json, current theme file, theme actions on App | app, color, fonts, store |
| `scripts/theme-switch.sh` | Switch palette | picker, writes i3, colors.sh, Ghostty, rofi, Pywalfox outputs, reloads | lib/paths.sh, lib/rofi-picker.sh |
| `scripts/font-switch.sh` | Switch font preset | picker, sizes, writes Ghostty, i3, rofi, lock, login screen, GTK, fontconfig outputs, reloads | lib/paths.sh, lib/rofi-picker.sh |
| `scripts/themes-switch.sh` | Switch theme | picker, runs font-switch then theme-switch | lib/paths.sh, lib/rofi-picker.sh |
| `scripts/lib/paths.sh` | Where scripts find things | data_file, share_file | none |
| `scripts/lib/rofi-picker.sh` | Shared rofi menu | json_picker, json_data, delete | none |
| `share/` | Data for scripts and studio | terminal-colors.json, palette-derive.jq, integrations.json, integrations/ (generators) | none |
| `packaging/nix/` | Nix package and modules | package, home-manager module, NixOS module, fonts | none |

Cycles: app ↔ {behaviour, draft, fonts_dl, fonts_preview, hit, prefetch, presets, settings, theme, tour_page, ui, …}; ui ↔ settings_ui, fonts_ui; fonts ↔ keymap, fonts_pick, font_tree; fonts_dl ↔ fonts_preview; paths ↔ store (tests only).
Key numbers: 35 Rust modules in 1 folder, 6 scripts, about 25k Rust lines, 12 files over 500 production lines, largest `app.rs` (2,448 production lines), 2 cycle groups with 22 of 35 modules (`scripts/dev/module-cycles.py`; corrected in the Phase 4 review from 21: the first script version cut each file at the first `#[cfg(test)]` and missed code after it), line coverage 82.72%, duplicate rate 1.13%.

## 3. Target architecture
- `share/integrations.json`: the data list. For each app: its theme and font outputs (file + generator) and its reload command.
- `share/integrations/`: one small jq, template, or hook file for each output.
- `scripts/lib/integrations.sh`: the runner. It writes the outputs and runs the reloads one time. It is the only location that runs reload commands.
- `scripts/lib/paths.sh` and `studio/src/paths.rs`: each user and system path, with one `home()`. The login-screen paths can be changed with environment variables.
- `studio/src/app/`: App divided by concern: `palette_editor`, `layout`, `messages`, `overlay`, `mouse`, `actions`, `nav` (Page, Overlay, For, Side).
- `studio/src/model/`, `system/`, `ui/`: folders by layer, 10 modules or fewer each. Direction: ui → app → model and system. No cycles.

Advanced techniques that the refactor adds:
- Module re-exports (`pub use`) | `main.rs` during the folder moves | keep `crate::x` paths valid, so one move changes 5 files or fewer. A comment explains each re-export.
- Hook generators | `share/integrations/*.sh` | GTK, fontconfig, and the login screen need logic that a jq file cannot express.

## 4. Findings
| # | File(s) | Problem | Target violated | Impact |
|---|---|---|---|---|
| 1 | `scripts/theme-switch.sh`, `font-switch.sh` | App integrations hard-coded, outputs and reloads mixed with picker logic | T2, T6, goal 1-2 | H |
| 2 | `studio/src/app.rs` | God object: 60 public fields, 6 concerns, 2,448 production lines | T1 (SRP), T4, limits | H |
| 3 | most modules ↔ `app.rs` | Dependency cycles, mostly for nav enums and one helper | T5 | H |
| 4 | `store.rs`, `fonts.rs`, `theme.rs`, `download.rs`, `paths.rs` | `home()` defined 5 times, user paths outside paths.rs | T2 (single source), rule 1 | M |
| 5 | `download.rs:544` (the `font-switch.sh:102-103` paths go with R.2) | Hard-coded system paths | goal 3, rule 1 | M |
| 6 | `ui.rs`, `keymap.rs`, `fonts.rs`, `settings.rs`, `config.rs` | Over 500 production lines (2,154, 1,370, 1,288, 1,071, 611) | limits | M |
| 7 | `settings_ui.rs`, `download.rs`, `tour_page.rs`, `behaviour.rs`, `fonts_dl.rs`, `samples.rs` | Over 500 production lines (819 to 522) | limits | L |
| 8 | `download.rs` | 18 ignored results (`let _ =`) | T7 | M |
| 9 | 191 functions | Over 30 lines (guideline) | T3 guideline | L (reason comments when changed) |
| 10 | all | 490 clippy pedantic warnings | goal 7 | M |
| 11 | all | No `# Errors` sections (51 functions return Result), rustdoc fails (3 errors) | goal 8 | M |
| 12 | 9 test sites | Data tests skip without real data (1 skip at run time) | goal 4 | M |
| 13 | `paths.rs:196` | Not rustfmt-formatted | readability | L |
| 14 | `scripts/*` | 14 shellcheck findings, no tests | rule 3, goal 4 | M |
| 15 | `samples/code/go.go:49` | Old dotfiles path in sample text | success criteria | L |
| 16 | `desktop.rs` test `real_configs_parse` | Looks for `i3/theme` in the data folder (`store.dir`), but the desktop configs are in `store.desktop`. The test never runs since the data moved (found in P0.5, not fixed) | correctness of tests | M |
| 17 | `scripts/themes-switch.sh` | A theme with a missing palette: the fonts are applied (files and `fonts/current` change), then the palette fails with exit 1, and nothing reloads. The desktop is left half-switched (found in P0.8, recorded in golden case `themes-switch-missing-palette`, not fixed) | correctness, T7 | M |
| 18 | `paths.rs` (`config_home`, `studio_config_dir`, `history_dir`, `timing_log_dir`) | The same XDG base folders are read three ways: `config_home` accepts any non-empty `XDG_CONFIG_HOME`; `studio_config_dir` and `history_dir` accept only absolute values; `timing_log_dir` accepts any `XDG_STATE_HOME`, even empty. Kept as they were (P1.2). | T2 (single source) | L |
| 19 | `scripts/lib/paths.sh`, `studio/src/paths.rs` | The two files agree on colors_dir, fonts_dir, host_fonts_file only through comments. No test compares them (Phase 1 review) | T2 (single source) | L |
| 20 | `studio/src/paths.rs` | `studio_config_dir`, `history_dir`, `timing_log_dir` have no tests for their different XDG rules (finding 18). Tests need care: env vars in parallel tests (Phase 1 review) | T4 (testability) | M |
| 21 | `config::dir()`, `history::default_dir()` | One-line wrappers around paths.rs functions since P1.2. Callers could use paths:: directly (Phase 1 review) | T4 | L |
| 22 | `studio/src/preview.rs:338` | Reads `XDG_RUNTIME_DIR` (else the temp folder) for the preview's runtime files outside paths.rs (found in P1.5) | rule 1 | L |
| 23 | `scripts/theme-switch.sh` (and `reload_apps`) | The exit code of a switch is the exit code of its last reload command. Without Pywalfox, theme-switch exits 127 although all files were written. A caller that checks the exit code (the studio, home-manager activation) sees a failed switch (found in P2.6, kept unchanged) | correctness, T7 | M |
| 24 | `draft.rs` `settings_save` | When the save fails, the draft is dropped and its values go live, although nothing was written (found in P3.1, recorded in a test, not fixed) | correctness, T7 | H |
| 25 | `draft.rs` `draft_changes` | Returns at least 1 even when only keys outside "settings" and "keymaps" differ (`n.max(1)`) (P3.1) | correctness | L |
| 26 | `behaviour.rs` | Confirm on a Gap row moves the cursor to row 0; Header rows are selectable (P3.1) | correctness | L |

## 5. Test coverage
- Rust: 169 tests in 27 modules (165 pass, 4 ignored). `app.rs` has about 2,500 lines of tests.
- No tests: `fonts_dl`, `fonts_dl_ui`, `fonts_preview`, `fonts_ui`, `settings_ui`, `tour_page`, `tour_ui`, `main`.
- Scripts: no tests → golden-file tests in Phase 0.
- Scripts: golden-file tests in `tests/scripts/run.sh` (P0.6 to P0.8). Not covered: the rofi picker.
- Nix sandbox: data tests use `studio/tests/fixtures/data` (P0.5). Sandbox skips: 13 before, 10 after. The remaining skips need jq, ImageMagick, fontconfig, a pty, or the checkout (docs, share).
- Before Phase 5 changes a module without tests, the revise action adds a safety-net step for it.

## 6. Plan

Step format (one line for each step):
`- [ ] P<phase>.<n> <title>: <what changes> | Files: <paths> | Target: <criterion or target> | Check: <how to test>`

### Phase 0: Foundation and safety net
- [x] P0.1 Dev tools: add clippy, rustfmt, shellcheck, cargo-modules, cargo-llvm-cov, and lizard to the dev shell | Files: `flake.nix` | Target: tools | Check: each tool runs in `nix develop`
- [x] P0.2 Phase 0 baselines: measure cycles, dependency direction, coverage, and duplicates. Record them | Files: `docs/refactor/SUCCESS_CRITERIA.md`, `docs/refactor/PLAN.md` (section 2 numbers) | Target: baselines | Check: no "Phase 0" left in the baselines
- [x] P0.3 Lint configuration: `[lints.clippy]` pedantic, fix rustfmt at `paths.rs:196` | Files: `studio/Cargo.toml`, `studio/src/paths.rs` | Target: goal 7 | Check: `cargo fmt --check` passes, clippy count = 490
- [x] P0.4 Flake checks: `checks.x86_64-linux` with build, tests, fmt, and ratchet counts for clippy (490) and shellcheck (14) | Files: `flake.nix`, `scripts/dev/lint-counts.sh`, `docs/refactor/lint-baseline.txt` | Target: goal 4, 7 | Check: `nix flake check` passes
- [x] P0.5 Test fixtures: small palettes.json, fonts.json, themes.json, configs, so the data tests run in the sandbox | Files: `studio/tests/fixtures/`, `studio/src/store.rs`, `studio/src/palette.rs`, `studio/src/paths.rs` | Target: goal 4 | Check: skip count does not increase, sandbox runs the data tests
- [x] P0.6 Golden tests, theme-switch: test runner with stub commands (pkill, i3-msg, rofi, fc-cache, pywalfox, ghostty), fixture data, golden files | Files: `tests/scripts/run.sh`, `tests/scripts/fixtures/`, `tests/scripts/golden/theme-switch/` | Target: goal 4 | Check: golden test passes on unchanged code
- [x] P0.7 Golden tests, font-switch: fonts, sizes, offsets. The login-screen part skips, and the skip is recorded | Files: `tests/scripts/run.sh`, `tests/scripts/golden/font-switch/`, `tests/scripts/fixtures/` | Target: goal 4 | Check: golden test passes on unchanged code
- [x] P0.8 Golden tests, themes-switch, and the scripts in flake checks | Files: `tests/scripts/run.sh`, `tests/scripts/golden/themes-switch/`, `flake.nix` | Target: goal 4 | Check: `nix flake check` runs the golden tests
Completes: criterion 4 (the check command exists)

### Phase R: Removals (goal change, 2026-11-26)
- [x] R.1 [removal] Wallpaper hook: theme-switch.sh no longer runs `wallpaper-rotate refresh` | Files: `scripts/theme-switch.sh`, `tests/scripts/run.sh` (stub list) | Target: goal (colours, themes, configured programs) | Check: golden files lose only the wallpaper-rotate call
- [x] R.2 [removal] Login screen in font-switch.sh: remove the greeter part. Remove the namespace protection from `tests/scripts/run.sh` (nothing writes to /var/lib/ascii-boot any more) | Files: `scripts/font-switch.sh`, `tests/scripts/run.sh` | Target: goal | Check: golden files unchanged (the part was skipped), `grep ascii-boot scripts/` = 0
- [x] R.3 [removal] NixOS module: remove `packaging/nix/nixos-module.nix` and the `nixosModules` output, and its documentation | Files: `packaging/nix/nixos-module.nix`, `flake.nix`, `README.md`, `docs/theme-studio.md` | Target: goal | Check: `nix flake check`
- [x] R.4 Shared data reader: move `json_data` from `rofi-picker.sh` into `scripts/lib/data.sh` (no behavior change) | Files: `scripts/lib/rofi-picker.sh`, `scripts/lib/data.sh`, `theme-switch.sh`, `font-switch.sh`, `themes-switch.sh` | Target: T1 | Check: golden files unchanged
- [x] R.5 [removal] rofi picker: remove `scripts/lib/rofi-picker.sh` and the picker branches. No name: a usage message, exit 2 | Files: `scripts/lib/rofi-picker.sh`, `theme-switch.sh`, `font-switch.sh`, `themes-switch.sh`, `tests/scripts/run.sh` (new no-name cases) | Target: goal | Check: golden files unchanged, new no-name cases pass
- [x] R.6 [removal] Picker documentation: README and the guide describe the switch commands with a name only | Files: `README.md`, `docs/theme-studio.md`, `packaging/nix/package.nix`, `studio/src/store.rs` | Target: goal | Check: `grep -i "rofi menu\|picker" README.md docs/theme-studio.md` names only rofi as a themed app
- [x] R.7 [removal] Neutral rofi menu in the preview: the preview's rofi window keeps rofi's colours and font, without the delete message, the Alt+BackSpace hint, and the "theme picker" wording | Files: `studio/src/preview.rs` | Target: goal (review finding, Phase R) | Check: cargo test, the preview still draws a rofi window
Completes: the removals of GOAL.md. Criterion 3 for the scripts (the system paths of font-switch.sh go with R.2)

### Phase 1: One source for paths
- [x] P1.1 One `home()` and the output folders in paths.rs: remove the 4 copies of `home()`, add `colors_dir`, `fonts_dir`, `host_fonts_file`, move the `/etc/fonts/fonts.conf` constant | Files: `paths.rs`, `store.rs`, `fonts.rs`, `theme.rs`, `download.rs` | Target: goal 3, T2, rule 1 | Check: tests
- [x] P1.2 Config and state folders through paths.rs | Files: `paths.rs`, `config.rs`, `history.rs`, `fonts.rs` | Target: rule 1 | Check: tests
- [x] P1.3 Script folders in paths.sh: `colors_dir`, `fonts_dir`, `cache_dir` | Files: `scripts/lib/paths.sh`, `theme-switch.sh`, `font-switch.sh`, `themes-switch.sh` | Target: goal 3 | Check: golden tests
- [x] P1.4 Sample text: replace the old dotfiles path in the Go sample with `$XDG_CONFIG_HOME/theme-studio/themes.json` | Files: `studio/src/samples/code/go.go` | Target: success criteria | Check: grep = 0, tests
- [x] P1.5 Download folders through paths.rs: move the `xdg()` helper of download.rs and its 3 folders (installed fonts, cache, preview cache) into paths.rs, behavior unchanged | Files: `studio/src/paths.rs`, `studio/src/download.rs` | Target: goal 3, rule 1 | Check: tests, no XDG or HOME path built outside paths.rs
Completes: criterion 3 for the Rust code. For the scripts, the per-app paths (GTK settings.ini, fontconfig conf.d, Ghostty themes) become entries of the integrations list in Phase 2, which completes criterion 3 there.

### Phase 2: App integrations as data
- [x] P2.1 Runner and list format: `integrations.json` with the i3 and rofi theme outputs. theme-switch uses the runner | Files: `share/integrations.json`, `scripts/lib/integrations.sh`, `scripts/theme-switch.sh`, `share/integrations/i3-colors.jq`, `share/integrations/rofi-colors.jq` | Target: goal 2, T2 | Check: golden tests unchanged
- [x] P2.2 colors.sh, Ghostty, and Pywalfox theme outputs in the list (Ghostty theme-line check as a hook; `share/pywalfox-colors.jq` moves to `share/integrations/` with `git mv`) | Files: `share/integrations.json`, `scripts/lib/integrations.sh`, `share/integrations/shell-colors.jq`, `share/integrations/ghostty-colors.sh`, `scripts/theme-switch.sh` | Target: goal 2 | Check: golden tests unchanged
- [x] P2.3 Ghostty, i3, and rofi font outputs in the list | Files: `share/integrations.json`, `scripts/font-switch.sh`, `share/integrations/ghostty-font.jq`, `i3-font.jq`, `rofi-font.jq`, `scripts/lib/integrations.sh` | Target: goal 2 | Check: golden tests unchanged
- [x] P2.4 Lock screen, GTK, and fontconfig font outputs as hooks | Files: `share/integrations.json`, `scripts/font-switch.sh`, `share/integrations/lock-font.sh`, `gtk-font.sh`, `fontconfig-font.sh`, `scripts/lib/paths.sh` | Target: goal 2 | Check: golden tests unchanged
- [x] P2.6 One reload location: reload commands come from the list, `reload_apps` runs them one time. themes-switch reloads once for both | Files: `share/integrations.json`, `scripts/lib/integrations.sh`, `scripts/theme-switch.sh`, `scripts/font-switch.sh` (`themes-switch.sh` unchanged) | Target: rule 2 | Check: golden reload logs unchanged
- [x] P2.7 Extensibility check: on a temporary branch, add a sample terminal. Record `git diff --stat` | Files: `docs/refactor/SUCCESS_CRITERIA.md` | Target: goal 1 | Check: 2 files or fewer, no script logic
Completes: criteria 1, 2, project rule 2

### Phase 3: Divide App by concern
- [x] P3.1 Safety net: characterization tests for behaviour and draft | Files: `behaviour.rs`, `draft.rs` | Target: correctness | Check: tests pass on unchanged code
- [x] P3.2 App folder: `app.rs` → `app/mod.rs`, tests → `app/tests.rs` | Files: `app/mod.rs`, `app/tests.rs` | Target: T4 | Check: tests
- [x] P3.3 PaletteEditor: palette, saved, undo, redo, HSV, hex, edit session, and their methods | Files: `app/mod.rs`, `app/palette_editor.rs`, `app/tests.rs`, `ui.rs`, `fonts_ui.rs`, `theme.rs`, `settings.rs`, `tour_page.rs`, `fonts_preview.rs` | Target: T1, T4 | Check: tests
- [x] P3.4 Layout: sizes, borders, dragging, resize, visibility flags, pane columns | Files: `app/mod.rs`, `app/layout.rs`, `app/tests.rs`, `behaviour.rs`, `draft.rs`, `ui.rs`, `fonts_ui.rs`, `fonts_dl_ui.rs`, `fonts_preview.rs`, `settings_ui.rs`, `tour_ui.rs` | Target: T1 | Check: tests
- [x] P3.5 Messages: message, notice, hover note, timing | Files: `app/mod.rs`, `app/messages.rs`, `app/tests.rs`, `ui.rs`, `fonts_ui.rs`, `behaviour.rs`, `draft.rs`, `fonts_dl.rs`, `fonts_preview.rs`, `presets.rs`, `settings.rs`, `theme.rs`, `tour_page.rs` | Target: T1 | Check: tests
- [x] P3.6 Overlay keys: `on_overlay_key` and overlay state | Files: `app/mod.rs`, `app/overlay.rs` | Target: T1, limits | Check: tests
- [x] P3.7 Mouse: `handle_mouse`, `click`, `on_mouse` | Files: `app/mod.rs`, `app/mouse.rs`, `app/tests.rs` | Target: T1, limits | Check: tests
- [x] P3.8 Key dispatch: `handle_key`, `run_action`, `on_key` | Files: `app/mod.rs`, `app/actions.rs`, `app/tests.rs` | Target: T1, limits | Check: tests, `app/mod.rs` under 500 production lines
- [x] P3.9 Saving and opening: save, save_as, apply, open, guard, run_pending, save_before, the palette preview sync → `app/persist.rs` (second impl block). The fonts save helpers (fonts_save, fonts_save_as, fonts_open_overlay) and notify_saved, reset_history stay beside them in persist.rs (they use App overlay and messages state, and show the repeated save pattern) | Files: `app/mod.rs`, `app/persist.rs` | Target: T1, limits | Check: tests, behavior unchanged
- [x] P3.10 Pane focus: focus_pane, move_pane, current_pane, picker enter/leave → `app/focus.rs`. run_resize stays with focus (it needs settings and current_pane) | Files: `app/mod.rs`, `app/focus.rs`, `app/tests.rs` (imports only) | Target: T1, limits | Check: tests
- [x] P3.11 Colour rows and text input: colour_rows, group_of, move_row, sync_row, set_group_open, list_action, on_hex_key, on_paste, cycle_focus → `app/colours.rs` | Files: `app/mod.rs`, `app/colours.rs`, `app/tests.rs` (imports only) | Target: T1, limits | Check: tests
- [x] P3.12 Nav enums: Page, Overlay, For, Side, Focus, Pending, ColourRow → `app/nav.rs` (replaces P4.1). Other modules import them from `app::nav` | Files: `app/mod.rs`, `app/nav.rs`, `app/tests.rs` (+ access-path edits in 16 files: app/{actions,colours,focus,mouse,overlay,persist}.rs, behaviour, fonts_dl, fonts_preview, fonts_ui, hit, prefetch, presets, settings, theme, tour_page, ui) | Target: T5 | Check: tests, `app/mod.rs` under 500 production lines, fewer cycles (check "fewer cycles" not met: 2 groups, 22 modules; carried over to Phase 4)
Inherited long functions in app/mod.rs (with_config, run, focus_pane, list_action, run_resize, on_hex_key) get their reason comments, or are divided, in the step that moves them.
Completes: none (prepares criterion 5)

### Phase 4: Dependency direction
- [x] P4.1 (replaced by P3.12, Phase 3 revision) Nav enums into `app/nav.rs`: Page, Overlay, For, Side, Pending, Focus | Files: `app/mod.rs`, `app/nav.rs`, `hit.rs`, `prefetch.rs`, `presets.rs` | Target: T5 | Check: tests, fewer cycles
- [x] P4.2 nav.rs as a leaf: move `Capture`, `Naming`, `Deferred` (settings.rs) and `BhRow` + `LookId` (behaviour.rs) into app/nav.rs as plain enums, so nav is in no cycle (it depends only on color and keymap; keymap does not depend on nav) | Files: `app/nav.rs`, `settings.rs`, `behaviour.rs` (+ access-path edits in 8 files: app/{mod,overlay,tests}.rs, fonts_dl, presets, settings_ui, tour_page, ui) | Target: T5 | Check: tests; module-cycles: nav, hit, prefetch leave the big group
- [x] P4.3 The run loop gets the draw function: App no longer calls `crate::ui::draw` (pass it in, or move the event loop to main.rs) | Files: `app/mod.rs`, `main.rs`, `docs/refactor/PLAN.md` | Target: T5 (direction ui -> app) | Check: tests; the drawing modules leave the big group
- [x] P4.4 `base64` out of app into the leaf module `encoding.rs` (app and download.rs use it) | Files: `app/mod.rs`, `app/tests.rs`, `download.rs`, `main.rs`, `encoding.rs` (new) | Target: T5 | Check: download no longer imports app
- [x] P4.5 prefetch: its `impl App` block moves to `app/prefetch_actions.rs` | Files: `prefetch.rs`, new `app/prefetch_actions.rs`, `app/mod.rs` | Target: T5 | Check: prefetch no longer imports App
- [x] P4.6 presets: `impl App` block into app/ | Files: `presets.rs`, new `app/presets_actions.rs`, `app/mod.rs`, `settings_ui.rs` | Target: T5 | Check: presets no longer imports App
- [x] P4.7 draft: `impl App` block into app/ | Files: `draft.rs`, new `app/draft_actions.rs`, `app/mod.rs` | Target: T5 | Check: draft no longer imports App
- [x] P4.8 theme: theme data stays in theme.rs, the `impl App` block (theme actions) moves into app/ | Files: `studio/src/theme.rs`, `studio/src/app/theme_actions.rs`, `studio/src/app/mod.rs` | Target: T1, T5 | Check: theme no longer imports App
- [x] P4.9 behaviour: `impl App` block into app/ | Files: `studio/src/behaviour.rs`, `studio/src/app/behaviour_actions.rs`, `studio/src/app/mod.rs` | Target: T5
- [x] P4.10 fonts_dl: `impl App` block into app/ | Files: `studio/src/fonts_dl.rs`, `studio/src/app/fonts_dl_actions.rs`, `studio/src/app/mod.rs` | Target: T5 | Check: fonts_dl no longer imports App
- [x] P4.11 fonts_preview: two `impl App` blocks into `app/fonts_preview_actions.rs` | Files: `fonts_preview.rs`, `app/fonts_preview_actions.rs`, `app/mod.rs` | Target: T5
- [x] P4.12 tour_page: `impl App` block into app/ (app/tour_actions.rs, app/tour_rounds.rs) | Files: `tour_page.rs`, `app/mod.rs`, `app/tour_actions.rs`, `app/tour_rounds.rs` | Target: T5
- [x] P4.13 settings: `impl App` block into app/ (app/settings_actions.rs, settings_keymaps_actions.rs, settings_history_actions.rs) | Files: `settings.rs`, `app/mod.rs`, the three new files | Target: T5 | Check: no feature module imports App; the big group is gone
- [x] P4.14 Cycle ratchet: `scripts/dev/module-cycles.py` in the flake checks, with the count after P4.13 as the baseline (it must never go up) | Files: `flake.nix`, `scripts/dev/module-cycles.py`, `docs/refactor/lint-baseline.txt` | Target: T5 | Check: `nix flake check`
For P4.5 to P4.13: one module per step; the app/mod.rs map (its `//!`) is updated in each step. Each step reports module-cycles before and after.
Completes: none (cycle count decreases, measured)

### Phase 5: Large files and folders (revised after Phase 4)
- [x] P5.0 app/ sub-folders: `app/state/` (palette_editor, layout, messages, nav), `app/input/` (actions, focus, mouse, overlay, colours), `app/pages/settings/`, `app/pages/fonts/`, `app/pages/themes/` (the *_actions files, without the suffix inside their folder); persist.rs and tests.rs stay at the top. Mostly `git mv` and mod lines; the app/mod.rs map follows | Files: app/ (moves) | Target: T5, audience | Check: tests; each folder 10 files or fewer
- [x] P5.1 `ui/widgets.rs` as a leaf: friendly and panel_number move there (ratatui only); pane_border and pane_title need App, so they go to `ui/pane_chrome.rs`; ui.rs becomes ui/mod.rs | Files: `ui.rs` -> `ui/mod.rs`, `ui/widgets.rs`, `widgets.rs`, users (access paths) | Target: T5 | Check: tests; the {ui, settings_ui, fonts_ui} cycle is gone
- [x] P5.2 ui: page drawing into `ui/pages.rs` | Files: `ui/mod.rs`, `ui/pages.rs` | Target: limits | Check: tests
- [x] P5.3a ui: layout and status line: `ui/layout.rs` (arrange, MainLayout, Arrangement, borders, constants), `ui/status.rs` (draw_status, fit_hints, draw_message_only) | Files: `ui/mod.rs`, the two new files | Target: limits | Check: tests
- [x] P5.3b ui: panes into `ui/panes/` (colours.rs: draw_list, draw_palette_menu; picker.rs; preview.rs: draw_preview, draw_swatches) | Files: `ui/mod.rs`, `ui/panes/*` | Target: limits | Check: tests
- [x] P5.3c ui: overlays into `ui/overlays/` (mod.rs: draw_pending, draw_notice, popup, centered; dialogs.rs: draw_overlay, divided by overlay kind) | Files: `ui/mod.rs`, `ui/overlays/*` | Target: limits | Check: tests, ui/mod.rs under 500
- [x] P5.4 ui/ complete: settings_ui, fonts_ui, fonts_dl_ui, tour_ui, preview move into ui/ (sub-folders if over 10 modules) | Files: `ui/page_bodies/{mod,settings,fonts_column,fonts_download,tournament_tab}.rs`, `ui/desktop_preview.rs`, `ui/mod.rs`, main.rs (+ access-path edits in 9 files) | Target: T5 | Check: tests; ui/ folders 10 modules or fewer
- [x] P5.5 `fonts/types.rs` as a leaf: CHIPS, ROLES, ROLE_NAMES, Installed, FontFocus, Row; keymap, fonts_pick, font_tree import the leaf, not fonts | Files: `fonts.rs` -> `fonts/mod.rs`, `fonts/types.rs`, keymap.rs, fonts_pick.rs, font_tree.rs | Target: T5 | Check: tests; the {keymap, fonts, fonts_pick, font_tree} cycle is gone
- [x] P5.6 keymap: default bindings as data, separate from keymap logic; `Action::group()` (Window, Theme, Fonts or Colours), so run_action reads the group from data (Phase 3 review, note 2) | Files: `keymap.rs` -> `keymap/mod.rs`, `keymap/defaults.rs`, app/input/actions.rs | Target: T2, limits | Check: tests
- [x] P5.7 fonts/: rendering jobs and installed fonts into their own files; fonts_pick, font_tree, and the data parts of fonts_dl and fonts_preview move into fonts/ | Files: fonts/ (moves and splits) | Target: T1, T5, limits | Check: tests; fonts/mod.rs under 500
- [x] P5.8 config: sizes and defaults separate from load and save | Files: `config.rs` -> `config/mod.rs`, `config/sizes.rs` | Target: limits | Check: tests
- [x] P5.11 desktop_preview split: the hidden-terminal state moved to `preview_term/mod.rs` (Preview, Mode, PaneId, draft palette) and `preview_term/pane.rs` (Pane, TermColors, key_bytes), outside ui; the drawing (bar, rofi, windows) stays in `ui/desktop_preview.rs`; each under 500 lines | Files: ui/desktop_preview.rs, preview_term/{mod,pane}.rs, main.rs (+ access-path edits in 6 files) | Target: limits, T5 | Check: tests; cycles 0
- [x] P5.9 `core/` folder (done): color, encoding, history, palette, paths, samples (with its data folder), settle, store, theme, tournament (10 modules; `config/` stays a root folder, to keep core at 10). Not moved: hit, behaviour, draft, presets, settings, tour_page, docs (screen code: no place yet, see P5.10 note) | Files: main.rs, core/mod.rs, moves | Target: T5 | Check: tests
- [x] P5.12 Rename `core/` to `model/` (decided after P5.9: `core` hides Rust's built-in core crate) | Files: the folder, access paths, docs
- [x] P5.13 Page state into folders (decided after P5.9): new `settings_page/` (mod from settings.rs, behaviour, draft, keymap_presets from presets.rs, readme from docs.rs); tour_page.rs to `fonts/tournament_page.rs`; hit.rs to `app/state/hit.rs` | Files: moves, access paths | Target: T5 | Check: tests; cycles 0
- [x] P5.10 `system/` folder: desktop, download, prefetch, out (samples went to model/ in P5.9; the page-state modules went to folders in P5.13), and the cycle ratchet set to 0: `nix flake check` fails on any cycle | Files: main.rs, system/mod.rs, moves, `docs/refactor/lint-baseline.txt` | Target: T5 | Check: tests; cycles 0; no module left in the root except main.rs
- [ ] P5.14a `ui/page_bodies/settings.rs` (844 production lines) under 500: divide by tab | Files: the file, new files next to it | Target: limits | Check: tests
- [ ] P5.14b `system/download.rs` (699) under 500 | Files: the file, new files in system/ | Target: limits | Check: tests
- [x] P5.14c `model/samples.rs` (523) under 500 | Files: the file, new files in model/ | Target: limits | Check: tests
- [ ] P5.14d `app/pages/themes/behaviour_actions.rs` (521) under 500 | Files: the file, new files next to it | Target: limits | Check: tests
Removed: the old "settings: tabs into separate files" step (Phase 4 moved the impl App blocks out; settings.rs is under 500).
Completes: criterion 5 (with the docs action), no dependency cycles

### Phase 6: Lint and errors to zero
Also: `palette_menu_focused` and `page_tabs_focused` may become one `enum TopFocus { None, PaletteMenu, PageTabs }` (check first that both are never true at once; Phase 3 review, note 3).
- [ ] P6.1 Clippy pedantic to zero in core | Files: core modules (5 or fewer per step, more steps if necessary) | Target: goal 7 | Check: count decreases
- [ ] P6.2 Clippy pedantic to zero in system | Files: system modules | Target: goal 7 | Check: count decreases
- [ ] P6.3 Clippy pedantic to zero in app | Files: app modules | Target: goal 7 | Check: count decreases
- [ ] P6.4 Clippy pedantic to zero in ui, then `-D warnings` as the gate | Files: ui modules, `flake.nix` | Target: goal 7 | Check: count = 0
- [ ] P6.5 Errors in download.rs: handle or report the 18 ignored results | Files: `system/download.rs` | Target: T7 | Check: tests, each `let _ =` has a reason or is gone
- [ ] P6.6 Shellcheck to zero | Files: `scripts/*.sh`, `scripts/lib/*.sh` | Target: rule 3 | Check: count = 0
Completes: criterion 7

### Phase 7: [feature] Install without Nix
- [ ] P7.1 [feature] Install script: `install.sh` with PREFIX: cargo build, scripts, share files, desktop entry | Files: `install.sh`, `packaging/theme-studio.desktop` | Target: goal 6 | Check: install into `$(mktemp -d)`, `command -v` finds each command
- [ ] P7.2 [feature] Dependencies and docs: README section with the install command and each runtime dependency | Files: `README.md`, `docs/theme-studio.md` | Target: goal 6 | Check: each program that the code starts is listed
- [ ] P7.3 [feature] Install test: golden tests against the installed commands, without THEME_STUDIO_SHARE, in flake checks | Files: `tests/scripts/run.sh`, `flake.nix` | Target: goal 6 | Check: `nix flake check`; `nix build` still passes
Completes: criterion 6

## 7. Out of scope
- **Preview per app (next feature after this refactor):** `desktop.rs` and `preview.rs` draw i3, rofi, i3blocks, and nvim. A new integration does not appear in the preview. The next feature makes the preview read `share/integrations.json`.
- Support for other Linux distributions: a later project (GOAL.md).
- Security hardening: before a public release (GOAL.md).
- Starter data in `share/` (migration notes): new behavior, not in the goal.
- X11 and kitty-protocol dependencies (`xrdb`, graphics protocol): part of the multi-distro project.
- `set -euo pipefail` in the scripts: it changes behavior.
- Splitting `settings_ui.rs`, `download.rs`, `tour_page.rs`, `behaviour.rs`, `fonts_dl.rs`, `samples.rs` (finding 7): a later plan revision decides.

**Future refactors** (not in this refactor; noted on 2026-11-26):
- **Terminal independence:** Theme Studio should work with terminals other than Ghostty. Isolate the terminal from the utility:
  - Configured terminals: after Phase 2, a terminal is one entry in `share/integrations.json` plus a generator. Check that nothing outside the list names Ghostty: `share/terminal-colors.json` (shared by the scripts and the preview, built around Ghostty's 16 colours and theme names), the Ghostty theme-folder check (`share/integrations/ghostty-colors.sh`), and the `pkill -USR2 ghostty` reload (P2.6 moves it into the list).
  - The terminal that the studio runs in: the studio uses the kitty graphics protocol for font previews (half blocks otherwise) and the terminal's own colours. Put this behind one small interface, so the studio does not assume the terminal that it runs in.
  - The preview draws a Ghostty-style terminal: part of the "preview per app" feature.
  - The preview terminal is one module since P5.11: `studio/src/preview_term/`. Ghostty-specific places there: `TermColors` in pane.rs (colours as Ghostty sets them, from terminal-colors.json); mod.rs sets TERM, COLORTERM and THEME_COLORS for the shell and uses the `colors.sh` draft file.

**Handover** (goal change, Phase R). After the removals, the dotfiles must provide these. No dotfiles change in this refactor.
- Wallpaper border colour (R.1): run `wallpaper-rotate refresh` after a theme switch, for example in a dotfiles wrapper around `theme-switch`, or a watch on `~/.config/colors/current`.
- Login screen (R.2, R.3): remove `services.theme-studio.loginFont` from the NixOS configuration. Set the greeter font in the dotfiles.
- rofi menus (R.5): menus that list the names in `palettes.json`, `fonts.json`, and `themes.json` (the data folder) and run `theme-switch <name>`, `font-switch <name>`, or `themes-switch <name>`. Replace the i3 key bindings (`$mod+t`, `$mod+p`, `$mod+Shift+t`) that ran the scripts without a name: they now get exit 2. Deleting a preset from a menu: the studio does it, or the dotfiles menu does.
- `docs/migration.md` and `docs/plan.md` describe the old setup (login screen, wallpaper, pickers). The docs action updates or archives them.

## 8. Outcome
<!-- The final action completes this section. -->

## 9. Architecture notes (pending)
<!-- The review action adds notes here. The revise action processes them. -->

## 10. Plan changelog
| Phase | Note | Decision | Reason |
|---|---|---|---|
| Goal | Theme Studio configures colours and themes for the system and the programs it is set up to configure: remove the wallpaper hook, the login screen (font-switch part, NixOS module), and the rofi picker menus | ACCEPT: new Phase R (R.1 to R.6), P1.3 without the login-screen paths, P2.5 removed | The user changed the goal on 2026-11-26. Removals first make Phases 1 and 2 smaller. |
| Goal | Wording: "colours and themes for the system, and the programs that it is set up to configure" (was "only fonts and colour palettes"). The three removals stay. | ACCEPT: no step changes | The user corrected the wording on 2026-11-26. The login screen goes because users of a future release can use different login tools. |
| R | Review: the preview still draws the removed rofi theme picker (delete message, Alt+BackSpace hint) | ACCEPT: new step R.7 | The user chose a neutral rofi menu on 2026-11-26. rofi stays a themed app. |
| 1 | Move the xdg() helper of download.rs and its 3 folders into paths.rs | ACCEPT: new step P1.5 | Criterion 3 for the Rust code. Small, no behavior change. |
| 1 | Rename one of the two fonts_dir functions | DONE in the review (download::installed_fonts_dir) | Audience test: same name, different meaning. |
| 2 | One place for the share folder and the state folders, also for the hooks (share_dir, gtk_settings_file, ghostty_themes_dir) | DONE in the review | Criterion 3 for the scripts. Small. |
| 3 | Divide the rest of app/mod.rs (1,294 lines) by concern | ACCEPT: P3.9 to P3.12; P4.1 replaced by P3.12 | Reviewer: right division for a junior developer; the nav enums step is required to reach 500 lines. |
| 3 | Action::group() in keymap.rs | DEFER to P5.4 | The flat match fixes the audience problem now; keymap.rs changes in P5.4 anyway. |
| 3 | TopFocus enum for the two top-row focus flags | DEFER to Phase 6 | Small; check first that both are never true. |
| 3 | Break the big cycle group: nav as a leaf, draw function passed in, impl App blocks of the feature modules into app/ | ACCEPT: Phase 4 replaced (P4.2 to P4.14) | P3.12 moved the nav enums but the cycles did not shrink (2 groups, 22 modules): Overlay names settings and behaviour types, and the feature modules impl App. |
| 4 | Revise Phase 5: app/ sub-folders first (P5.0), ui/widgets leaf, fonts/types leaf, folder lists complete, cycles to 0 | ACCEPT: Phase 5 rewritten (P5.0 to P5.10) | Phase 5 was written before Phases 3 and 4; the two remaining cycles break on splits that Phase 5 does anyway. |
| 5 | P5.3 is too large (panes ~700, overlays ~520 lines) | ACCEPT: divided into P5.3a, P5.3b, P5.3c (same goal) | Found in P5.2; the three parts take different functions out of ui/mod.rs. |
| 6 | ui/desktop_preview.rs (about 1000 lines) holds terminal state and drawing | ACCEPT: new step P5.11 before P5.9 | Found in P5.4. |
| 7 | After P5.9: `core` hides Rust's core crate; 7 page-state modules have no place in P5.10 | USER: rename to `model/` (P5.12); page-state folder `settings_page/`, tour_page to fonts/, hit to app/state/ (P5.13) | Decided by the user. |
| 8 | After P5.10: 4 production files are still over the 500-line hard limit | ACCEPT: P5.14a-d, one file each, in parallel | Phase 5 has the limits as its target. |

## 11. Progress
<!-- The kit updates this table after each action. -->
| Item | Status |
|---|---|
| Plan | done |
| Phase 0 safety net | done |
| Phase R steps | done |
| Phase R review | done |
| Phase 1 steps | done |
| Phase 1 review | done |
| Phase 2 steps | done |
| Phase 2 review | done |
| Phase 3 steps | done |
| Phase 3 review | done |
| Phase 4 steps | done |
| Phase 4 review | done |
| Phase 5 steps | open |
| Phase 5 review | open |
| Phase 6 steps | open |
| Phase 6 review | open |
| Phase 7 steps | open |
| Phase 7 review | open |
| Documentation | open |
| Final review | open |
