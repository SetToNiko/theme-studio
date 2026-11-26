# Before and after: Theme Studio

<!-- The refactor-kit final action wrote this file. "Before" comes from section 2 of PLAN.md. "After" comes from the code at the end of the refactor. -->

Goal: After the refactor, a junior developer can read the codebase, quickly find the purpose of each module, and understand the architecture of the project. The code is as easy as possible to maintain and to extend.

## Key numbers

| Measure | Before | After |
|---|---|---|
| Modules | 35 Rust modules in 1 folder, plus 6 scripts | 122 `.rs` files under `studio/src` (114 production files; 8 test files in `studio/src/app/tests/`), plus 6 scripts (3 switch scripts, 3 in `scripts/lib/`), `install.sh`, 2 test runners, and 4 dev scripts in `scripts/dev/` |
| Source lines | about 25,000 Rust lines (scripts: not recorded) | 30,919 Rust lines; 419 script lines in `scripts/*.sh` and `scripts/lib/*.sh` |
| Files over the file limit (500 production lines) | 12 | 0 (the flake check `file-limits` enforces it) |
| Largest file (production lines) | `app.rs`, 2,448 | `studio/src/app/input/actions.rs`, 474 |
| Dependency cycles | 2 groups, 22 of 35 modules | 0 groups, 0 modules |
| Tests | 115 pass, 4 ignored | 191 pass, 4 ignored; plus 22 golden script cases and the install test |
| Clippy warnings (pedantic, style, correctness) | 490 | 0 |
| Shellcheck findings | 14 | 0 |
| Line coverage | 82.72% | 82.75% |
| Functions over 30 lines (lizard, with the signature; tests included) | 191 of 809, no reasons | 137 of 1,269; 73 long-function reasons, each checked by a reviewer (0 unverified) |
| `# Errors` sections on functions that return `Result` | 3 of 53 | all (gate `errors-doc`) |

How the numbers were counted: production lines are the lines before `#[cfg(test)]` followed by `mod tests`. Test files (`app/tests/`) do not count as files over the limit. The cycle count comes from `scripts/dev/module-cycles.py`.

## Before

| Module | Purpose | Responsibilities | Became |
|---|---|---|---|
| `studio/src/main.rs` | Entry point | terminal setup, event loop start, module list | `main.rs` (unchanged path) |
| `app.rs` | Editor state and input handling | 60-field App state, palette editing + undo, pane layout + resize, overlay dialogs, key dispatch, mouse, fonts tab, themes, messages | `app/mod.rs`, `app/state/`, `app/input/`, `app/persist.rs`, `app/pages/` |
| `ui.rs` | Drawing | all pages, panes, overlays, status line | `ui/` (`mod.rs`, `pages.rs`, `layout.rs`, `panes/`, `overlays/`, `status/`, `widgets.rs`, `pane_chrome.rs`) |
| `keymap.rs` | Keymaps | action names, contexts, bindings, keymap files | `keymap/` (`mod.rs`, `action.rs`, `table.rs`, `defaults.rs`, `key.rs`) |
| `fonts.rs` | Fonts column of the Main page | font presets, installed fonts (fc-list), sample rendering jobs, DPI (xrdb) | `fonts/` (`mod.rs`, `presets.rs`, `installed.rs`, `render.rs`, `types.rs`, `filters.rs`) |
| `settings.rs` | Settings page (F3) | Keymaps, Behaviour, History tabs, draft handling | `settings_page/mod.rs`, `app/pages/settings/settings_*_actions.rs` |
| `settings_ui.rs` | Drawing the Settings page | tab strip, keymaps, behaviour, history views | `ui/page_bodies/settings/` |
| `download.rs` | Downloading fonts | Fontsource and Nerd Fonts lists, downloads, preview fontconfig | `system/download/` |
| `tour_page.rs` | Tournament tab | pool choice, preview fetch, rounds | `fonts/tournament_page.rs`, `app/pages/fonts/tour_actions.rs`, `app/pages/fonts/tour_rounds.rs` |
| `config.rs` | The studio's own settings | settings.json, sizes, load and save | `config/mod.rs`, `config/settings.rs` |
| `behaviour.rs` | Behaviour tab | settings values in groups, editing | `settings_page/behaviour.rs`, `app/pages/settings/behaviour_*_actions.rs` |
| `fonts_dl.rs` | Download fonts page (F2) | tabs, search, install actions | `fonts/download_page.rs`, `app/pages/fonts/fonts_dl_actions.rs` |
| `samples.rs` | Preview texts for fonts | sample texts, ImageMagick rendering | `model/samples.rs`, `model/samples/langs.rs`, `fonts/render.rs` |
| `preview.rs` | Live preview | model i3 desktop, real programs (nvim) in a PTY | `ui/desktop_preview.rs`, `ui/panes/preview.rs`, `preview_term/` |
| `fonts_dl_ui.rs` | Drawing the Download page | tabs, chips, lists | `ui/page_bodies/fonts_download.rs`, `fonts_download_details.rs`, `fonts_download_preview.rs` |
| `palette.rs` | A palette | 34 keys, base-colour rules, jq reading | `model/palette.rs` |
| `history.rs` | Version history of config files | snapshots, restore, dates | `model/history.rs` |
| `hit.rs` | Clickable areas | hit map, borders, hover | `app/state/hit.rs` |
| `tournament.rs` | Font tournament | pairing, results | `model/tournament.rs` |
| `fonts_ui.rs` | Drawing the fonts column | roles, installed list | `ui/page_bodies/fonts_column.rs`, `ui/page_bodies/fonts_roles.rs` |
| `fonts_preview.rs` | Fonts page preview | draw a font before install, menus | `fonts/preview_page.rs`, `app/pages/fonts/fonts_preview_actions.rs` |
| `docs.rs` | README tab | find and render docs/theme-studio.md | `settings_page/readme.rs`, `ui/page_bodies/settings/readme.rs` |
| `presets.rs` | Keymap preset browser | preset list, apply | `settings_page/keymap_presets.rs`, `app/pages/themes/presets_actions.rs` |
| `desktop.rs` | The preview's desktop model | read real i3, rofi, i3blocks configs, run bar blocks | `system/desktop.rs` |
| `tour_ui.rs` | Drawing the Tournament tab | setup, rounds, results | `ui/page_bodies/tournament_tab.rs` |
| `prefetch.rs` | Fetch font previews early | background queue | `system/prefetch.rs`, `app/pages/fonts/prefetch_actions.rs` |
| `store.rs` | Files the studio reads and writes | palettes.json, current palette, JSON writes, own `home()` | `model/store.rs` (`home()` now only in `model/paths.rs`) |
| `font_tree.rs` | Font list tree | group families into two levels | `fonts/tree.rs` |
| `fonts_pick.rs` | Fonts for roles | kind of font for each role | `fonts/pick.rs` |
| `paths.rs` | Where things are | data, share, scripts, desktop configs, switch command | `model/paths.rs` |
| `draft.rs` | Unsaved Settings changes | draft keymaps and behaviour | `settings_page/draft.rs`, `app/pages/settings/draft_actions.rs` |
| `color.rs` | Colour type | hex, HSV, HSL, contrast | `model/color.rs` |
| `settle.rs` | Image size settling | wait until a pane size is stable | `model/settle.rs` |
| `out.rs` | Terminal output | frame buffering | `system/out.rs` |
| `theme.rs` | Themes | theme data, themes.json, current theme file, theme actions on App | `model/theme.rs`, `app/pages/themes/theme_actions.rs` |
| `scripts/theme-switch.sh` | Switch palette | picker, writes i3, colors.sh, Ghostty, rofi, Pywalfox outputs, reloads | `scripts/theme-switch.sh` (the picker is removed; the outputs moved to `scripts/lib/integrations.sh` and `share/integrations/`) |
| `scripts/font-switch.sh` | Switch font preset | picker, sizes, writes Ghostty, i3, rofi, lock, login screen, GTK, fontconfig outputs, reloads | `scripts/font-switch.sh` (the picker and the login screen are removed; the outputs moved to `scripts/lib/integrations.sh` and `share/integrations/`) |
| `scripts/themes-switch.sh` | Switch theme | picker, runs font-switch then theme-switch | `scripts/themes-switch.sh` (the picker is removed) |
| `scripts/lib/paths.sh` | Where scripts find things | data_file, share_file | `scripts/lib/paths.sh` (unchanged path; now the only script place for system paths) |
| `scripts/lib/rofi-picker.sh` | Shared rofi menu | json_picker, json_data, delete | removed (Phase R: rofi menus) |
| `share/` | Data for scripts and studio | terminal-colors.json, palette-derive.jq, integrations.json, integrations/ (generators) | `share/` (unchanged path; the data list and the generators now drive all outputs) |
| `packaging/nix/` | Nix package and modules | package, home-manager module, NixOS module, fonts | `packaging/nix/` (the login-screen part of the NixOS module is removed in Phase R) |

Also removed in Phase R: the wallpaper hook.

## After

| Module | Purpose | Responsibilities | Came from |
|---|---|---|---|
| `studio/src/main.rs` | Entry point | Starts the terminal and the event loop. | `main.rs` |
| `studio/src/app/mod.rs` | The `App` struct | Holds the editor state and its methods. | `app.rs` |
| `studio/src/app/run.rs` | Main loop | Runs until quit; refreshes and polls before each draw; sends terminal events to the key, mouse and paste handlers. | `app.rs` |
| `studio/src/app/persist.rs` | Save and apply | Saves, opens and applies palettes and font presets. | `app.rs` |
| `studio/src/app/tests/` (`mod.rs`, `font_downloads.rs`, `input.rs`, `layout.rs`, `lifecycle.rs`, `pages_fonts.rs`, `pages_settings.rs`, `palette.rs`) | Tests for the app | Test-only files, one for each area: font downloads, input, layout, lifecycle, Fonts page, Settings page, palette. | `app.rs` |
| `studio/src/app/input/` (`mod.rs`, `actions.rs`, `colours.rs`, `focus.rs`, `mouse.rs`, `overlay.rs`) | Input handling | Key dispatch and actions, colour rows, pane focus, mouse, overlay keys. | `app.rs` |
| `studio/src/app/state/` (`mod.rs`, `hit.rs`, `layout.rs`, `messages.rs`, `nav.rs`, `palette_editor.rs`, `steps.rs`) | Editor data | Clickable areas, pane layout, messages, navigation, palette undo, list cursors. | `app.rs`, `hit.rs` |
| `studio/src/app/pages/fonts/` (`mod.rs`, `fonts_dl_actions.rs`, `fonts_preview_actions.rs`, `prefetch_actions.rs`, `tour_actions.rs`, `tour_rounds.rs`) | App methods of the Fonts page | Download page, preview, prefetch, tournament set-up and rounds. | `fonts_dl.rs`, `fonts_preview.rs`, `prefetch.rs`, `tour_page.rs`, `app.rs` |
| `studio/src/app/pages/fonts/fonts_install_actions.rs` | Font install actions | Install and remove fonts on the Download page; install results; Nerd Font offer. | `fonts_dl.rs` |
| `studio/src/app/pages/settings/` (`mod.rs`, `settings_actions.rs`, `settings_history_actions.rs`, `settings_keymaps_actions.rs`, `behaviour_actions.rs`, `behaviour_colours_actions.rs`, `behaviour_labels_actions.rs`, `draft_actions.rs`) | App methods of the Settings page | Page keys, History, Keymaps and Behaviour tabs, the draft. | `settings.rs`, `behaviour.rs`, `draft.rs` |
| `studio/src/app/pages/themes/` (`mod.rs`, `theme_actions.rs`, `presets_actions.rs`) | App methods for themes | Open, save, apply themes; the keymap preset browser. | `theme.rs`, `presets.rs` |
| `studio/src/config/mod.rs`, `settings.rs` | The studio's own settings | Loads and saves settings.json and keymaps.json; defines the settings data. | `config.rs` |
| `studio/src/fonts/mod.rs` | Fonts column of the Main page | Font state of the Main page. | `fonts.rs` |
| `studio/src/fonts/presets.rs` | Font presets | Reads and writes fonts.json. | `fonts.rs` |
| `studio/src/fonts/installed.rs` | Installed fonts | Lists families from `fc-list`. | `fonts.rs` |
| `studio/src/fonts/render.rs` | Sample image | Builds Pango markup; draws it with ImageMagick in background jobs. | `fonts.rs`, `samples.rs` |
| `studio/src/fonts/types.rs`, `filters.rs`, `pick.rs`, `tree.rs` | Font helpers | Leaf types, filter chips, kind of font per role, two-level font tree. | `fonts.rs`, `fonts_pick.rs`, `font_tree.rs` |
| `studio/src/fonts/download_page.rs`, `preview_page.rs`, `tournament_page.rs` | Page state of the Fonts feature | Download page (F2), font preview, Tournament tab. | `fonts_dl.rs`, `fonts_preview.rs`, `tour_page.rs` |
| `studio/src/keymap/mod.rs` | Keymaps | Contexts, bindings, keymap files. | `keymap.rs` |
| `studio/src/keymap/action.rs`, `table.rs` | Actions | Action identity and the one table of ids, labels and groups. | `keymap.rs` |
| `studio/src/keymap/defaults.rs`, `key.rs` | Keys | Shipped presets (data); key parsing and display in Vim notation. | `keymap.rs` |
| `studio/src/model/color.rs` | Colour type | Hex, HSV, HSL, WCAG contrast. | `color.rs` |
| `studio/src/model/encoding.rs` | Base64 | Standard alphabet with padding. | `download.rs` (`app` base64) |
| `studio/src/model/history.rs` | Version history | Snapshots and restore of the config files. | `history.rs` |
| `studio/src/model/palette.rs` | A palette | 34 keys, base-colour rules, derive rules. | `palette.rs` |
| `studio/src/model/paths.rs` | Where things are | The one Rust place for system paths. | `paths.rs`, `download.rs`, `store.rs` |
| `studio/src/model/samples.rs`, `samples/langs.rs`, `samples/code/rust.rs` | Preview texts | Pango markup samples; language table; Rust sample source (no header). | `samples.rs` |
| `studio/src/model/settle.rs` | Image size settling | Waits until a pane size is stable. | `settle.rs` |
| `studio/src/model/store.rs` | Studio files | Reads and writes palettes.json and the current palette. | `store.rs` |
| `studio/src/model/theme.rs` | Themes | Theme data and themes.json. | `theme.rs` |
| `studio/src/model/tournament.rs` | Font tournament | Pairing and results. | `tournament.rs` |
| `studio/src/model/mod.rs` | Model folder | Data and rules; no screen code. | new |
| `studio/src/preview_term/mod.rs`, `pane.rs` | Live terminal | Hidden programs (nvim) in a PTY with a draft palette. | `preview.rs` |
| `studio/src/settings_page/mod.rs` | Settings page (F3) | Tabs and the draft rule. | `settings.rs` |
| `studio/src/settings_page/behaviour.rs`, `draft.rs` | Behaviour tab and draft | Setting values in groups; unsaved changes. | `behaviour.rs`, `draft.rs` |
| `studio/src/settings_page/keymap_presets.rs`, `readme.rs` | Keymap presets and README tab | Preset browser; finds and renders the user guide. | `presets.rs`, `docs.rs` |
| `studio/src/settings_page/readme_layout.rs` | README text layout | Word wrapping of styled text and table layout in columns. | `docs.rs` |
| `studio/src/system/desktop.rs` | Preview desktop model | Reads real i3, rofi and i3blocks configs. | `desktop.rs` |
| `studio/src/system/download/` (`mod.rs`, `catalogue.rs`, `install.rs`, `list.rs`, `net.rs`, `preview.rs`) | Font download | Font lists, install job, installed list file, curl calls, preview cache. | `download.rs` |
| `studio/src/system/switch.rs` | Switch-script runner | The only place that starts `theme-switch` and `font-switch`; tests never run them. | `paths.rs` (`switch_command`), `app.rs` |
| `studio/src/system/out.rs`, `prefetch.rs`, `mod.rs` | Terminal and prefetch | Frame output; early font previews; folder for system-facing code. | `out.rs`, `prefetch.rs` |
| `studio/src/ui/mod.rs`, `pages.rs`, `layout.rs`, `widgets.rs`, `pane_chrome.rs` | Drawing core | Entry, whole pages, pane layout, shared pieces, pane borders. | `ui.rs` |
| `studio/src/ui/desktop_preview.rs` | Desktop preview drawing | Bar, rofi menu, two windows. | `preview.rs` |
| `studio/src/ui/overlays/` (`mod.rs`, `dialogs.rs`, `help.rs`) | Overlays | Dialogs, key list, hints, save notice. | `ui.rs` |
| `studio/src/ui/panes/` (`mod.rs`, `colours.rs`, `picker.rs`, `preview.rs`) | Main-page panes | Palette, colour picker, preview. | `ui.rs`, `preview.rs` |
| `studio/src/ui/status/` (`mod.rs`, `rows.rs`) | Status line | Tabs, message, key hints. | `ui.rs` |
| `studio/src/ui/page_bodies/` (`mod.rs`, `rows.rs`, `fonts_column.rs`, `fonts_roles.rs`, `fonts_download*.rs`, `tournament_tab.rs`) | Fonts page drawing | Fonts column, roles, Download page, Tournament tab. | `fonts_ui.rs`, `fonts_dl_ui.rs`, `tour_ui.rs` |
| `studio/src/ui/page_bodies/settings/` (`mod.rs`, `behaviour.rs`, `history.rs`, `keymaps.rs`, `readme.rs`) | Settings page drawing | Tab strip and the four tabs. | `settings_ui.rs` |
| `scripts/theme-switch.sh` | Switch colour scheme | Applies a palette. | `scripts/theme-switch.sh` |
| `scripts/font-switch.sh` | Switch font preset | Applies a font preset. | `scripts/font-switch.sh` |
| `scripts/themes-switch.sh` | Switch theme | Applies a palette and a font preset together. | `scripts/themes-switch.sh` |
| `scripts/lib/paths.sh` | Where scripts find things | The one script place for system paths. | `scripts/lib/paths.sh` |
| `scripts/lib/data.sh` | Preset files | Reads palettes.json, fonts.json, themes.json. | new (from `rofi-picker.sh` data helpers and the switch scripts) |
| `scripts/lib/integrations.sh` | App outputs | Writes the outputs that `share/integrations.json` lists. | the switch scripts |
| `share/integrations.json`, `share/integrations/` | App integration data | One entry per app; one generator file per output (11 files). | switch scripts, old `share/` |
| `scripts/dev/file-limits.sh` | File-size gate | Prints each `.rs` file over 500 production lines; used by the flake check `file-limits`. | new |
| `scripts/dev/errors-doc.sh` | `# Errors` gate | Checks that each production function that returns `Result` has a `# Errors` section; used by the flake check `errors-doc`. | new |
| `scripts/dev/lint-counts.sh`, `scripts/dev/module-cycles.py` | Ratchets | Lint warning count that never goes up; module cycle count. | new |
| `install.sh` | Install without Nix | Installs the commands and share files into a prefix. | new |
| `tests/scripts/run.sh` | Golden-file tests | Tests the switch scripts. | new |
| `tests/install/run.sh` | Install test | Installs to a temporary prefix, tests, uninstalls. | new |
| Flake checks in `flake.nix` | Checks | `build`, `fmt`, `clippy`, `doc`, `shellcheck`, `cycles`, `errors-doc`, `file-limits`. | new |

## In short
To make Theme Studio configure a new program, you add one entry to `share/integrations.json` and one generator file in `share/integrations/`. No script logic changes. The old `app.rs` (about 2,450 production lines) and the other flat files now live in folders by job: `app/`, `ui/`, `model/`, `system/`, `fonts/`, `keymap/`, `settings_page/`, `config/` and `preview_term/`. `model/` is a clean leaf: it runs no programs and imports nothing above it, and `system/switch.rs` is the only place that starts the switch scripts. The flake checks keep the limits: no cycles, no file over 500 production lines, no warnings, and a `# Errors` section on each function that returns `Result`. Phase R removed the wallpaper hook, the login screen and the rofi menus.
