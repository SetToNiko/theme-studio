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
Key numbers: 35 Rust modules in 1 folder, 6 scripts, about 25k Rust lines, 12 files over 500 production lines, largest `app.rs` (2,448 production lines), 2 cycle groups with 21 of 35 modules (`scripts/dev/module-cycles.py`), line coverage 82.72%, duplicate rate 1.13%.

## 3. Target architecture
- `share/integrations.json`: the data list. For each app: its theme and font outputs (file + generator) and its reload command.
- `share/integrations/`: one small jq, template, or hook file for each output.
- `scripts/lib/integrations.sh`: the runner. It writes the outputs and runs the reloads one time. It is the only location that runs reload commands.
- `scripts/lib/paths.sh` and `studio/src/paths.rs`: each user and system path, with one `home()`. The login-screen paths can be changed with environment variables.
- `studio/src/app/`: App divided by concern: `palette_editor`, `layout`, `messages`, `overlay`, `mouse`, `actions`, `nav` (Page, Overlay, For, Side).
- `studio/src/core/`, `system/`, `ui/`: folders by layer, 10 modules or fewer each. Direction: ui → app → core and system. No cycles.

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

## 5. Test coverage
- Rust: 119 tests in 25 modules (115 pass, 4 ignored). `app.rs` has about 2,500 lines of tests.
- No tests: `behaviour`, `draft`, `fonts_dl`, `fonts_dl_ui`, `fonts_preview`, `fonts_ui`, `settings_ui`, `tour_page`, `tour_ui`, `main`.
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
- [ ] P3.1 Safety net: characterization tests for behaviour and draft | Files: `behaviour.rs`, `draft.rs` | Target: correctness | Check: tests pass on unchanged code
- [ ] P3.2 App folder: `app.rs` → `app/mod.rs`, tests → `app/tests.rs` | Files: `app/mod.rs`, `app/tests.rs` | Target: T4 | Check: tests
- [ ] P3.3 PaletteEditor: palette, saved, undo, redo, HSV, hex, edit session, and their methods | Files: `app/mod.rs`, `app/palette_editor.rs`, `ui.rs`, `fonts_ui.rs` | Target: T1, T4 | Check: tests
- [ ] P3.4 Layout: sizes, borders, dragging, resize, visibility flags, pane focus and move | Files: `app/mod.rs`, `app/layout.rs`, `ui.rs`, `fonts_ui.rs`, `fonts_dl_ui.rs` | Target: T1 | Check: tests
- [ ] P3.5 Messages: message, notice, hover note, timing | Files: `app/mod.rs`, `app/messages.rs`, `ui.rs`, `fonts_ui.rs` | Target: T1 | Check: tests
- [ ] P3.6 Overlay keys: `on_overlay_key` and overlay state | Files: `app/mod.rs`, `app/overlay.rs`, `ui.rs` | Target: T1, limits | Check: tests
- [ ] P3.7 Mouse: `handle_mouse`, `click`, `on_mouse` | Files: `app/mod.rs`, `app/mouse.rs` | Target: T1, limits | Check: tests
- [ ] P3.8 Key dispatch: `handle_key`, `run_action`, `on_key` | Files: `app/mod.rs`, `app/actions.rs` | Target: T1, limits | Check: tests, `app/mod.rs` under 500 production lines
Completes: none (prepares criterion 5)

### Phase 4: Dependency direction
- [ ] P4.1 Nav enums into `app/nav.rs`: Page, Overlay, For, Side, Pending, Focus | Files: `app/mod.rs`, `app/nav.rs`, `hit.rs`, `prefetch.rs`, `presets.rs` | Target: T5 | Check: tests, fewer cycles
- [ ] P4.2 `base64` out of app into `encoding.rs` | Files: `app/mod.rs`, `encoding.rs`, `download.rs`, `main.rs` | Target: T5 | Check: download no longer imports app
- [ ] P4.3 Theme data separate from theme actions: `theme.rs` keeps data, `app/theme_actions.rs` gets the App methods | Files: `theme.rs`, `app/theme_actions.rs`, `app/mod.rs` | Target: T1, T5 | Check: theme no longer imports app
- [ ] P4.4 Cycle check: cargo-modules acyclic check in the flake checks, with the current cycle count as a ratchet | Files: `flake.nix`, `scripts/dev/lint-counts.sh`, `docs/refactor/lint-baseline.txt` | Target: T5 | Check: `nix flake check`
Completes: none (cycle count decreases, measured)

### Phase 5: Large files and folders
- [ ] P5.1 ui.rs: page drawing into `ui/pages.rs` | Files: `ui.rs` → `ui/mod.rs`, `ui/pages.rs` | Target: limits | Check: tests
- [ ] P5.2 ui: pane drawing into `ui/panes.rs` | Files: `ui/mod.rs`, `ui/panes.rs` | Target: limits | Check: tests
- [ ] P5.3 ui: overlay drawing into `ui/overlays.rs` | Files: `ui/mod.rs`, `ui/overlays.rs` | Target: limits | Check: tests
- [ ] P5.4 keymap: default bindings as data, separate from keymap logic | Files: `keymap.rs` → `keymap/mod.rs`, `keymap/defaults.rs` | Target: T2, limits | Check: tests
- [ ] P5.5 fonts: rendering jobs into `fonts/render.rs`, installed fonts into `fonts/installed.rs` | Files: `fonts.rs` → `fonts/mod.rs`, `fonts/render.rs`, `fonts/installed.rs` | Target: T1, limits | Check: tests
- [ ] P5.6 settings: tabs into separate files | Files: `settings.rs` → `settings/mod.rs`, `settings/keymaps.rs`, `settings/history.rs` | Target: limits | Check: tests
- [ ] P5.7 config: sizes and defaults separate from load and save | Files: `config.rs` → `config/mod.rs`, `config/sizes.rs` | Target: limits | Check: tests
- [ ] P5.8 `core/` folder: color, palette, theme, store, history, paths, settle (re-exports in main.rs) | Files: `main.rs`, `core/mod.rs`, moved files | Target: T5 | Check: tests
- [ ] P5.9 `system/` folder: desktop, download, prefetch, samples, encoding | Files: `main.rs`, `system/mod.rs`, moved files | Target: T5 | Check: tests
- [ ] P5.10 `ui/` folder complete: fonts_ui, fonts_dl_ui, settings_ui, tour_ui, preview, hit, docs | Files: `main.rs`, `ui/mod.rs`, moved files | Target: T5 | Check: tests, each folder 10 modules or fewer
Completes: criterion 5 (with the docs action)

### Phase 6: Lint and errors to zero
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
| Phase 3 steps | open |
| Phase 3 review | open |
| Phase 4 steps | open |
| Phase 4 review | open |
| Phase 5 steps | open |
| Phase 5 review | open |
| Phase 6 steps | open |
| Phase 6 review | open |
| Phase 7 steps | open |
| Phase 7 review | open |
| Documentation | open |
| Final review | open |
