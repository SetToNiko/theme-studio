# Contributing to Theme Studio

<!-- The final action of the refactor kit wrote this file from the code at the end of the refactor (main at 068da69). It used the asd-ste100 skill in strict mode. Keep this file true. Write each change with the asd-ste100 skill. A change that moves a boundary, adds a module, or changes a gate also changes this file, in the same commit. -->

This file is for two readers:
- a person who contributes to the project
- a contributor skill or agent team that reads this file to plan parallel work

Each rule here is a rule of this project. A **gate** is a rule that a command checks. A **review rule** is a rule that a reviewer checks. A change must pass all gates and all review rules. To see a walk-through of the code and a glossary of the techniques, read [ARCHITECTURE.md](ARCHITECTURE.md).

## 1. Commands

| Job | Command |
|---|---|
| Development environment (cargo, clippy, rustfmt, shellcheck, python3, lizard, cargo-modules, cargo-llvm-cov) | `nix develop` |
| Build the package | `nix build` |
| Build the studio only | `cd studio && cargo build` (in `nix develop`) |
| Rust tests | `cd studio && cargo test` (in `nix develop`) |
| Script golden tests | `bash tests/scripts/run.sh` (in `nix develop`) |
| Install test | `bash tests/install/run.sh` (in `nix develop`) |
| **All gates** (run before each commit) | `nix flake check` |
| One gate | `nix build .#checks.x86_64-linux.<gate>` |
| Counts while you work (clippy, shellcheck, cycles) | `scripts/dev/lint-counts.sh` (in `nix develop`). Its shellcheck count covers only `scripts/*.sh` and `scripts/lib/*.sh`. The gate covers more files. |

Use the exit code of a command to decide. Do not use words in its output.

## 2. Gates

`nix flake check` runs all of them. A change that fails one gate is not done.

| Gate | What fails it | How to fix a failure |
|---|---|---|
| `build` | The package does not build, or a Rust test fails (`cargo test` runs in the build). | Fix the code or the test. Change an expected test value only in a `[bug fix]`, `[feature]`, `[removal]` or `[behavior]` step. Change it only for that change. |
| `fmt` | `rustfmt --check` finds a difference. | `cd studio && cargo fmt`. |
| `clippy` | Any warning of `cargo clippy --all-targets -- -D warnings`. `studio/Cargo.toml` adds the pedantic group. Style and correctness are the default groups of clippy. | Fix the code. To allow a lint, use `#[expect(<lint>, reason = "...")]` on the smallest item, with a true reason. |
| `doc` | Any rustdoc warning (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`). Example: a `<word>` outside backticks. | Put code and `<...>` in backticks. Fix broken intra-doc links. |
| `shellcheck` | Any finding in `scripts/*.sh`, `scripts/lib/*.sh`, `scripts/dev/*.sh`, `install.sh`, `tests/scripts/run.sh`, `tests/install/run.sh`. | Fix the script. Put a comment with the true reason on the line before each `# shellcheck disable=SCxxxx`. |
| `cycles` | A dependency cycle between Rust modules (`scripts/dev/module-cycles.py`). The gate names each module by its full path. It counts each `crate::` path in production code. | Move the shared item into a leaf file that both modules can use. Or reverse the dependency (section 4). |
| `file-limits` | A Rust file with more than 500 production lines (`scripts/dev/file-limits.sh`). Production lines are the lines before `#[cfg(test)]` followed by `mod tests`. Files named `tests.rs` and files in a `tests/` folder do not count. | Divide the file by topic. Aim for 450 lines or fewer. |
| `errors-doc` | A production function that returns `Result` and has no `# Errors` section in its doc comment (`scripts/dev/errors-doc.sh`). | Add `# Errors` with the conditions that make the function fail. Read the conditions from the code. |
| `scripts` | A golden case of `tests/scripts/run.sh` gives a different result. | Fix the script. Update a golden case (`--update <case>`) only for an intended behavior change. Check the diff. |
| `install` | `tests/install/run.sh` fails in one of these steps: install, the installed commands, a second install, or the uninstall. | Fix `install.sh`. |

Baselines: `docs/refactor/lint-baseline.txt` has 0 for clippy, shellcheck and cycles. Never raise a number.

## 3. Review rules

A reviewer checks these rules. No command checks them. Priority when two rules conflict: correctness > audience test > readability > maintainability, modularity, error handling > extensibility.

### Audience
The reader is a junior developer who knows Rust and bash basics. The test: a reader opens a file, understands what it does and why in a few minutes, and can change it safely. Each advanced technique has these items:
- a plain interface in front of it
- a comment with its name, the reason and how to use it
- an entry in the glossary of ARCHITECTURE.md

Each implicit framework behavior has a comment that points to the place where the code wires it.

### Architecture
- Dependency direction: `ui → app → model, system, and the feature folders` (`fonts`, `keymap`, `config`, `settings_page`, `preview_term`). Nothing outside `ui` uses `ui` in production code. `main.rs` passes `ui::draw` to `App::run`, so `app` does not depend on `ui`.
- `model/` is the leaf: data and rules. It starts no program. It uses no network. It imports nothing from another module in production code. A boundary trait keeps it free of a module above it (example: `model::theme::PresetSizes`, which `fonts::presets::FontStore` implements).
- `system/` talks to the world outside the studio: programs, the network, the terminal. It uses only `model`. `system/switch.rs` is the only place that starts `theme-switch` and `font-switch`.
- Feature folders use `model`, `system`, and leaf files of other feature folders (example: `fonts/types.rs`, which `keymap` uses). They do not use `app` or `ui`. (Known exception: section 7.)
- All `impl App` blocks stay in `app/`. No other module adds methods to `App`.
- Each module has one reason to change (SRP). Each module folder holds 10 modules or fewer, not counting `mod.rs`.
- No dependency cycles (gate `cycles`).
- No `pub use` re-exports. Each access path names the real module.
- Indirection: 3 jumps or fewer from the entry of a feature to its work. Nesting: 4 levels or fewer. Prefer plain loops and `if` over dense iterator chains.

### Size limits
| Item | Limit | Kind | Exception |
|---|---|---|---|
| Rust file | 500 production lines | hard (gate `file-limits`) | none |
| Module folder | 10 modules, not counting `mod.rs` | hard (review) | none |
| Function | 30 body lines | guideline | a true reason comment (next section) |
| Function | 100 lines | clippy `too_many_lines` | `#[expect(clippy::too_many_lines, reason = "...")]` |
| Step or commit | 5 files | guideline | A pure move (`git mv`), new test-data files, and files whose only change is an access path do not count. |

### Long functions
- A function with more than 30 body lines needs `// Long function: <reason>.` above it. The reason names a real constraint, for example "a pure data table: one row per key". Another example is "the order of the calls is the order on screen". A reason that only says what the function does is not a reason. Divide the function.
- A reviewer checks each reason. When the reviewer confirms a reason, the marker becomes `// Long function (reason verified, phase <N>): <reason>.`
- A reviewer does not check a verified function again, unless the function grows or an architecture rewrite of its module occurs. A change that makes a verified function longer changes its marker back to the unverified form.

### Comments and documentation
- Each `.rs` file starts with `//!`: a one-line purpose, then details. Each script starts with a `#` purpose comment.
- Each item has `///`: a one-sentence summary first, then details.
- Use these sections:
  - `# Errors` on each function that returns `Result` (gate `errors-doc`)
  - `# Panics` if bad input from a caller can make the function panic
  - `# Examples` if the use is not obvious
  - `# Safety` on `unsafe`
- Use backticks for code and intra-doc links (`` [`Type`] ``) for types. Define each term that a junior developer may not know.
- Comments say why. The code says what. No commented-out code.
- Use British spelling, as in the code (colour, behaviour).

### Code rules
- Data-driven: app integrations, palettes, presets and key actions are data, not branches. Keep one source of truth for each.
- When a table replaces a `match`, keep one exhaustive `match` with no default arm (example: `Action::has_table_row`).
- Paths: get each system path from `studio/src/model/paths.rs` or `scripts/lib/paths.sh`. The two files must agree (`colors_dir`, `fonts_dir`, `host_fonts_file`).
- Reloads: only `reload_apps` in `scripts/lib/integrations.sh` runs app reload commands.
- Errors: each message says what failed and what to do. An error keeps its cause (anyhow `.context`). An ignored result has a comment with the reason. Or a helper handles it, and the name of the helper says so (example: `notify` and `remove_file_if_present` in `system/download/mod.rs`).
- Casts: use `From` or `into` where the conversion is lossless. Use `try_from` with a clear fallback where a value can be too large. Use `#[expect(clippy::cast_..., reason = "...")]` only where you prove the range.
- No new dependency without the maintainer's approval.
- Scripts: the switch scripts (`scripts/*.sh`, `scripts/lib/*.sh`) do not use `set -euo pipefail` (it would change their behavior). `install.sh`, `scripts/dev/` and `tests/` use it. Use kebab-case file names and snake_case functions.

### Tests
- Rust: put an inline `#[cfg(test)] mod tests` at the end of each file. For `App` scenarios, use the folder `app/tests/`. Test names are snake_case and state the behavior.
- Scripts: a golden case in `tests/scripts/run.sh` for each script behavior. To add a new case, do these steps in this order:
  1. Add a `run_case <name> <command>` line.
  2. Add an optional `setup_<name>` function.
  3. Run `--update <name>`.
  4. Review the diff.
- A new behavior gets a test. A bug fix gets a test that fails before the fix.
- Tests never touch the real system. Tests use a temporary `HOME`, `THEME_STUDIO_DATA` and `PREFIX`. The switch scripts never run in Rust tests (`system::switch` returns "off in tests").
- A characterization test can record a bug. Only a `[bug fix]` step changes its expected value. It changes the value only for that bug.

### Commits
- Each commit is atomic: one `git revert` undoes it completely and leaves a working project (all gates pass).
- If one commit is not possible, use a change group. Each commit has the trailer `Change-Group: <name> (<n>/<total>)`. Add ", first" and ", last" to the trailer of the outer commits.
- A refactor commit does not change behavior. Tags allow exceptions:
  - `[feature]` (new behavior, with tests and documentation)
  - `[removal]` (removes a behavior and its documentation and tests)
  - `[bug fix]` (fixes one bug, with a test)
  - `[behavior]` (an intended change of existing behavior, which the commit message lists)

## 4. Codebase map

### Modules

| Module | Job | Public API (main items) | Depends on | Used by |
|---|---|---|---|---|
| `studio/src/main.rs` | The entry point: arguments (`--fonts`, `--new`, `--init-config`, `-h`), `history::init`, the terminal setup, `App::run(terminal, ui::draw)`. | the `theme-studio` command | app, ui, model, system, config, fonts | (none) |
| `app/` | The `App` struct (the whole editor state) and every `impl App` block: keys, mouse, timers, the page logic. | `App` (`new`, `with_config`, `run`), `Start`, `state::hit` (`Target`, `HitMap`, `looks`), `state::nav` (`Page`, `Overlay`, `Deferred`, `BhRow`, ...), methods that ui reads (`colour_rows`, `contexts`, `looks`, `draft_changes`, ...) | model, system, keymap, fonts, settings_page, preview_term, config | main, ui, settings_page (section 7) |
| `ui/` | Draws each frame from `App` with ratatui. Records the click and hover areas. | `ui::draw` only | app, model, system, fonts, keymap, config, settings_page, preview_term | main (tests of app use it too) |
| `model/` | The data and rules: colours, palettes, themes, history, tournament, preview text, paths. No screen. No programs. | `color::Rgb`, `palette::{KEYS, Palette, derive}`, `store::{Store, write_json_file, TerminalMap}`, `theme::{Theme, ThemeStore, ROLES, PresetSizes}`, `history::{init, History}`, `paths::*`, `samples::*`, `settle::Settle`, `encoding::base64`, `tournament::Tournament` | (nothing) | every module |
| `system/` | The world outside: the switch scripts, font downloads (curl, tar, fc-*), the real desktop configs, the terminal output. | `switch::{apply_theme, apply_fonts}`, `download::{Msg, Source, Entry, Catalogue, channel_pair}`, `download::install::{install, remove_files}`, `prefetch::Prefetch`, `desktop::Roles`, `out::Out` | model | main, app, ui, fonts, preview_term |
| `fonts/` | The Fonts page state: installed fonts, tree and filters, presets (`fonts.json`), role picking, sample rendering, the download, preview and tournament page state. | `FontsTab`, `types::{Installed, Row, CHIPS, ROLE_NAMES}`, `filters::*`, `presets::FontStore`, `render::SampleRenderer`, `download_page::DlPage`, `preview_page::FontPreview`, `tournament_page::TourPage` | model, system, keymap (`Action`) | app, ui, config, keymap (`types.rs` only), main |
| `keymap/` | Every key action, the key bindings for each context, and the keymap presets. | `action::{Action, ActionGroup, LegacyId, all_actions}`, `key::{Key, display_seq}`, `Context`, `Keymap` (`lookup`, `from_json`, `to_json`, `builtin`, `vim`, ...). The action table `table::ROWS` is private to keymap. | model (`theme::ROLES`), fonts (`types.rs` leaf) | app, ui, config, fonts, settings_page |
| `config/` | Loads `keymaps.json` and `settings.json`, adds missing items, migrates them and saves them. Holds the settings types. | `load(dir, write) -> Loaded`, `save_settings`, `KEYMAPS_VERSION`, `settings::{Settings, SETTINGS_VERSION, Sizes, ...}` | keymap, fonts (`filters`, `presets::OLD_ROLES`), model | main, app, ui (`Sizes`), settings_page |
| `settings_page/` | The Settings page state: the Keymaps, Behaviour, History and README tabs, and the draft. | `SettingsPage`, `Tab`, `draft::Draft`, `behaviour::behaviour_rows`, `keymap_presets::{keymap_diff, draw_browse}`, `readme::{Doc, render}` | config, keymap, model, app (section 7) | app, ui |
| `preview_term/` | The live pseudo-terminals behind the desktop preview (nvim, bash, yazi) and the draft colours file. | `Preview` (`new`, `sync_draft`, `send_key`, `send_paste`, `toggle_yazi`, `take_dirty`), `PaneId`, `Mode`, `pane::{Pane, TermColors, key_bytes}` | model, system (`desktop`) | app, ui |
| `scripts/` | `theme-switch`, `font-switch`, `themes-switch`, and `lib/` (`paths.sh`, `data.sh`, `integrations.sh`: the one runner of the integrations list). | the three commands (section "Script API") | `share/` | system/switch.rs, the user, i3 key bindings |
| `share/` | Data: `integrations.json` (the program list), `integrations/*` (generators), `palette-derive.jq`, `terminal-colors.json`. | the integrations schema (below) | (nothing) | scripts, model (`store::TerminalMap` reads `terminal-colors.json`. `palette.rs` is a Rust port of `palette-derive.jq`.) |
| `install.sh`, `packaging/` | Install without Nix (`PREFIX`, `DESTDIR`, `--uninstall`), the Nix package, the home-manager module, the desktop entry. | `./install.sh`, `homeManagerModules.default`, `packages.default` | scripts, share, studio | users |
| `tests/` | `tests/scripts/run.sh` (22 golden cases), `tests/install/run.sh`. Rust tests are next to their code. | (none) | scripts, install.sh | the flake checks |
| `scripts/dev/` | The gate scripts: `module-cycles.py`, `file-limits.sh`, `errors-doc.sh`, `lint-counts.sh`. | (none) | reads `studio/src` | the flake checks |

### Dependency graph
```
main ──► ui ──► app ──► fonts, keymap, config, settings_page, preview_term, system, model
ui   ──► the same modules as app, directly (fonts, keymap, config, settings_page, preview_term, system, model)
main ──► config, fonts, system, model  (start-up)

fonts         ──► system, keymap (Action), model
keymap        ──► fonts/types.rs (leaf), model
config        ──► keymap, fonts (filters, presets::OLD_ROLES), model
settings_page ──► config, keymap, model   (and app: the exception in section 7)
preview_term  ──► system (desktop), model
system        ──► model
model         ──► (nothing: the leaf)

system/switch.rs ──► scripts/*-switch ──► scripts/lib/integrations.sh ──► share/integrations.json + generators
```
The cycle gate counts per file. A folder can use a leaf file of another folder, and that other folder can use the first folder back. Example: `fonts` uses `keymap::action`. `keymap` uses only the leaf `fonts/types.rs`.

### How the modules communicate
| From | To | How | What crosses the boundary |
|---|---|---|---|
| main | app | function call | `App::run(terminal, ui::draw)`: the run loop calls `draw` for each frame |
| ui | app | `&mut App` | ui reads the state and writes the frame state back: `app.hits` (the hit map), `redraw_at`, `layout.palette_pane`, `messages.hover_note`. Another crossing is `hit::set_looks` (a process-wide global). |
| app | system/switch | function call, then a child process | `apply_theme(store, name)` and `apply_fonts(...)`. Environment: `THEME_STUDIO_DATA`, `THEME_STUDIO_SHARE`, `FONT_SIZES`, `NO_RESTART`. |
| system/download, prefetch | app | threads and one `mpsc` channel of `download::Msg` | download progress, install results with a font-cache warning, previews, the prefetch queue. `app/pages/fonts` reads them in `poll_*`. |
| fonts/render | app, ui | a worker thread, an `mpsc` channel, an `Arc<AtomicBool>` dirty flag | rendered sample images |
| preview_term | ui | reader threads per pane, `Arc<Mutex<vt100::Parser>>`, a dirty flag | the screens of nvim, bash and yazi |
| preview_term | nvim, bash, yazi | pseudo-terminals and the file `$XDG_RUNTIME_DIR/theme-studio-<pid>-<n>/colors.sh` | the draft palette (`THEME_COLORS`), key presses, pastes |
| system/desktop | i3blocks commands | child processes in their own process group, one thread each | bar block lines |
| config | files | `keymaps.json`, `settings.json` in the config folder | settings and keymaps. Warnings come back in `Loaded.warnings`. |
| model/store, history | files | JSON written through a temporary file and a rename. History snapshots come first. | palettes, fonts, themes, history |
| scripts | desktop programs | config files in `~/.config/colors/`, `~/.config/fonts/`, fontconfig and GTK, then reloads | the generated configs. `colors/current`, `colors/theme`, `fonts/current` |
| system/out | the terminal | stdout | frames (the code rewrites rapid blink to a double underline) |

### API boundaries
- **ui:** the only entry is `ui::draw(frame, &mut App)`. It first clears the frame state. Then it fills the frame state again. The order of the draw calls is the order on screen (tabs, page, status, overlays). `ui/widgets.rs` depends only on ratatui. The studio chrome uses named terminal colours. Only swatches and the preview use the palette that the user edits.
- **app:** a key press starts at `app/input/actions.rs` `run_action`, which routes by `Action::group()`. `guard(p)` runs an action at once. When there are unsaved changes, it asks first. `steps::wrapped` needs a length above 0. `state/nav.rs` is plain data.
- **model:** call `history::init` once at start (`main.rs`). Tests leave it unset, or give a test its own folder with `history::init_for_test`. Then tests never write to the real state folder. `store::write_json_file` records the old content in the history before it writes. Change `paths.rs` and `scripts/lib/paths.sh` together.
- **system:** in `system/download/`, run background commands with `low` (`nice -n 10`). Send thread messages with `notify` (the receiver can be gone). `switch_command` is private. Callers use `apply_theme` and `apply_fonts`. Under `cfg(test)` the switch scripts never run, unless `THEME_STUDIO_REAL_SWITCH` is set. When a palette switch follows, pass `quiet` (`NO_RESTART`) to `apply_fonts`. Then i3 and Ghostty restart once.
- **keymap:** text fields (hex, search, names) do not use the keymap. A pane looks first in its own context, then in the context of the column, then in Global. No key may both run an action and start a longer sequence. Shipped presets have no clashes and no Alt keys. Each Vim-bound action has keys in all five presets.
- **config:** if a file does not parse, the code leaves the file alone. It uses the defaults and gives a warning. `write = false` never writes.
- **settings_page:** edits of Keymaps and Behaviour go to the draft (`settings_page.draft`). The page writes nothing until Save. Discard drops the draft. If the user quits with a draft, the page asks first. A failed save keeps the draft and the old live values. History acts at once. Each write uses the history.
- **preview_term:** a pane starts on the first `ensure`, at its display size. Headless mode (tests) starts no program. The code sends `pending_input` only after the pane has output.
- **scripts:** `theme-switch <palette>`, `font-switch <preset>`, `themes-switch <theme>`. Exit codes:
  - 2 without a name
  - 1 for an unknown name or a list error
  - 0 otherwise

  An app reloads only if it has `reload` and an output for the running switch. `NO_RESTART` skips all reloads. The switch skips a missing reload program and writes a line on stderr. A failed reload is a warning. The switch still exits 0 when it writes the files. A hook writes through a temporary file and `cat >`. A hook never uses `mv` over the target.

### The integrations list (`share/integrations.json`, version 1)
`{version, apps[]}`. An app has `name` and `outputs[]`. It can also have these optional fields:
- `reload` (an argv array, no shell)
- `reload_output` (`show`, `hide`, `hide-all`)
- `note`

An output has these fields:
- `switch` (`theme` or `fonts`)
- `kind` (`jq`, `hook`, `script`)
- `dir` (`colors_dir`, `cache_dir`, `fonts_dir`, `fontconfig_dir`)
- `file`
- `generator` (a file in `share/integrations/`)

A `script` output is for `fonts` only. It has no `dir` and no `file` (a theme output of kind `script` fails with "unknown kind"). The order of the list is the reload order.

Generator contracts (`scripts/lib/integrations.sh`):
- jq theme: `jq -r --arg n NAME -f gen DATA`
- jq fonts: `jq -j -f gen FONTS_FILE`
- hook theme: `bash hook NAME DATA SHARE`
- hook fonts: `bash hook FONTS_FILE SHARE`
- script (fonts only): `bash script FONTS_FILE SHARE`. The script writes its own files.

The stdout of a hook becomes the file.

### Extension points
| To add | Change these files | Notes |
|---|---|---|
| A program that Theme Studio configures | `share/integrations.json` (one entry). One generator in `share/integrations/`. A stub and cases in `tests/scripts/run.sh`, and their golden folders in `tests/scripts/golden/` (`--update <case>`). `install.sh` `warn_missing` and docs/theme-studio.md, if the program needs a tool. | `integrations.sh` needs no change for an existing folder. A new state folder also needs a function in `scripts/lib/paths.sh` and a line in `output_folder` (`scripts/lib/integrations.sh`). It also needs `model/paths.rs` if the studio reads the folder. |
| A key action | `keymap/action.rs` (the variant and the `has_table_row` arm). The build fails without the arm. One row in `keymap/table.rs` (`all_actions` and the Settings list come from it). Keys in `Keymap::vim()` (`keymap/defaults.rs`). The other four presets derive from it. Add an override in a derived preset (also in `keymap/defaults.rs`) only if its key differs. The handler where its group routes the action: `app/input/actions.rs` (`run_action`), `app/pages/settings/settings_actions.rs` (`run_settings`), `app/pages/fonts/fonts_dl_actions.rs` (`run_downloads`), `app/input/focus.rs` (resize) | Groups: `Page`, `Window`, `Theme`, `Column`. A renamed action needs an arm in `Action::legacy`. `keymaps.json` needs no migration. Loading adds the missing actions. |
| A setting | The field and its default in `config/settings.rs`. A Behaviour row: `BhRow` (`app/state/nav.rs`), `behaviour_rows` (`settings_page/behaviour.rs`), label and value (`app/pages/settings/behaviour_labels_actions.rs`), edit (`behaviour_actions.rs`) | Serde defaults fill a new field. A renamed or removed value needs a rule in `Settings::normalise` (it runs on each load). The code only writes `SETTINGS_VERSION`. Nothing reads it. |
| A font download source | `system/download/mod.rs` (`Source`), `catalogue.rs` (parse), `install.rs` (install and its match arm, the only exhaustive match), `fonts/download_page.rs` (`DlTab`). `prefetch.rs` and `preview.rs`, if the source has previews. Also check the `Source` matches with a `Nerd` or `_` fallback in `app/pages/fonts/` (`fonts_dl_actions.rs`, `fonts_preview_actions.rs`, `fonts_install_actions.rs`) and `fonts/preview_page.rs`. They compile, but they treat a new source like Google. | |
| A background message | `system/download/mod.rs` (`Msg`), a handler in `app/pages/fonts/fonts_dl_actions.rs` (`poll_downloads`) | |
| A page | a folder in `app/pages/`, a `Page` variant (`app/state/nav.rs`). A page action (`Action::Page...`, see "A key action") with its arm in `run_view_action` (`app/input/actions.rs`). Click targets (`app/state/hit.rs`, `app/input/mouse.rs`). A body in `ui/page_bodies/`. The call in `ui/pages.rs`. A tab in `ui/mod.rs` | |
| A pop-up | an `Overlay` variant, a handler in `app/input/overlay.rs`, the drawing in `ui/overlays/` | A confirm step needs a `Deferred` variant and a case in `run_deferred`. |
| A Main pane | a file in `ui/panes/`, the call in `ui/pages.rs`, its rectangle in `ui/layout.rs` | |
| A preview program | `PaneId` and the arms in `preview_term/mod.rs` (`pane_mut`, `pane_command`, `ensure`), the drawing in `ui/desktop_preview.rs` | |
| A palette key | `model/palette.rs` (`KEYS`, `GROUPS`, `TEXT_KEYS`, `sources`, `derive`) and `share/palette-derive.jq` (the test `derive_matches_palette_derive_jq` compares the two). `ui/layout.rs` (`LIST_FULL` counts 34 keys). The number 34 in ARCHITECTURE.md, docs/theme-studio.md and the comments of `palette.rs`. | `KEYS` is `[&str; 34]`. Change the size. |
| A sample language | a file in `model/samples/code/`, an entry in `model/samples/langs.rs` | `LANGS` has a fixed size. |
| A data file version | `model/store.rs` (`DATA_VERSION`, `FONTS_VERSION`, `migrate_data_file`), `model/theme.rs` (`THEMES_VERSION`), `config/mod.rs` (`KEYMAPS_VERSION`, `migrate`) | |

## 5. Parallel work

A contributor skill uses this section to divide a change into steps for parallel agents.

### Ownership units
Two agents can change two different units at the same time, if neither changes a shared file of the other.
| Unit | Files | Notes |
|---|---|---|
| ui drawing | `studio/src/ui/` | Only `ui::draw` is public. A change inside ui touches no other module. |
| app pages | one folder of `studio/src/app/pages/` | The pages share `app/state/nav.rs` and `app/input/actions.rs`. |
| app input and state | `studio/src/app/input/`, `studio/src/app/state/` | All pages share them. Run one step at a time. |
| model | `studio/src/model/` | All modules use it. A signature change touches many files. Prefer additions. |
| system | `studio/src/system/` | Fonts and app share `download/mod.rs` (`Msg`, `Source`). |
| fonts | `studio/src/fonts/` | keymap, ui and `app/input` use `types.rs`. |
| keymap | `studio/src/keymap/` | A new action also touches `app/input/actions.rs`. |
| config | `studio/src/config/` | A new setting also touches the Behaviour files. |
| settings_page | `studio/src/settings_page/` | |
| preview_term | `studio/src/preview_term/` | `ui/desktop_preview.rs` uses the same fields (section 7, N1). |
| scripts and data | `scripts/`, `share/` | The golden files of `tests/scripts/` change with them. |
| install and packaging | `install.sh`, `packaging/`, `tests/install/` | |
| documentation | `README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `docs/theme-studio.md` | |

### Shared files
Many changes touch these files. In one wave, a maximum of one step can change each shared file. Prefer one wiring step after the wave.
- `studio/src/main.rs`, each `mod.rs`: the module lists.
- `studio/src/app/mod.rs` (the `App` fields), `app/state/nav.rs` (all page enums), `app/state/hit.rs` (`Target`), `app/input/actions.rs` (`run_action`), `app/input/mouse.rs`, `app/input/overlay.rs`, `app/tests/mod.rs` (the test helpers).
- `studio/src/keymap/action.rs`, `keymap/table.rs`, `keymap/defaults.rs`.
- `studio/src/config/settings.rs`, `settings_page/behaviour.rs`.
- `studio/src/system/download/mod.rs`, `studio/src/model/paths.rs` with `scripts/lib/paths.sh`.
- `share/integrations.json`, `tests/scripts/run.sh`, `tests/scripts/golden/`.
- `flake.nix`, `studio/Cargo.toml`, `studio/Cargo.lock`: run one step alone in its wave.
- `docs/theme-studio.md` "Runtime dependencies" and `install.sh` `warn_missing`: change them together.

### Conflict areas
These areas caused merge conflicts during the refactor.
- Two steps that divide one large file: each step moves only its own functions. At the merge, do these actions:
  - Remove both sides of each old-code block.
  - Keep all `mod` lines.
  - Remove the closing braces that git puts outside the markers.

  Then build the project and remove the unused imports.
- Import lines: a module move changes the `use` lines of many files. Run moves one after the other. Do not run them in parallel.
- Test modules: two steps that add tests to the same test file conflict at the end of the file. Put the tests in different files, or in different waves.
- A folder move and a split of the same files conflict in the import lines of the moved files. Run them in different waves.
- `docs/refactor/lint-baseline.txt` and `docs/refactor/PLAN.md`: only the coordinator changes them, one merge at a time.

### Agent roles
| Role | Job | Reads | Writes |
|---|---|---|---|
| Implementer | One step in one ownership unit | this file, the step | the files of its unit |
| Test writer | Tests for one unit before or after a change | this file, the unit | the tests of its unit |
| Reviewer | Checks the review rules of section 3, and each long-function reason | this file, the diff | nothing |
| Docs writer | Keeps this file, ARCHITECTURE.md and the user guide true | this file, the diff | the documents |
| Coordinator | Divides the work into waves, merges, runs the gates | everything | the plan, the merges |

### Merge protocol
1. Each step works in its own git worktree and branch.
2. The coordinator merges one step at a time with `--no-ff`. The coordinator runs `nix flake check` after each merge. The coordinator uses the exit code to decide.
3. A step that fails a gate after the merge goes back to its agent. Other steps continue.
4. An agent does not commit and does not edit the plan. The coordinator commits each step, with the step's message and tag.

### Safety rules for agents
- Run the project only in a temporary folder. Before you run a script that installs, removes, or writes files, set each of these variables to a temporary folder:
  - `HOME`
  - `XDG_CONFIG_HOME`
  - `XDG_DATA_HOME`
  - `XDG_STATE_HOME`
  - `XDG_CACHE_HOME`
  - `XDG_RUNTIME_DIR`
  - `THEME_STUDIO_DATA`
  - `PREFIX`
  - `DESTDIR`

  Point `THEME_STUDIO_SHARE` at the `share/` folder of the checkout, or leave it unset.
- Never run `theme-switch`, `font-switch` or `themes-switch` on the real desktop. Use `tests/scripts/run.sh`, which runs them with a temporary `HOME` and a stub `PATH`.
- Never run `./install.sh --uninstall` without an explicit temporary `PREFIX`. The default `PREFIX` (`~/.local`) holds the real preview fonts of the studio.

## 6. Definition of done
- All gates of section 2 pass (`nix flake check` exits 0).
- The review rules of section 3 hold for the changed code.
- New behavior has a test and documentation (docs/theme-studio.md for users, ARCHITECTURE.md for contributors).
- Section 4 of this file is still true. If the change moves a boundary, adds a module or changes a gate, change this file in the same commit.

## 7. Open items
- **settings_page uses app.** `settings_page/behaviour.rs` uses `app::state::nav::{BhRow, LookId}` (and adds `impl BhRow`). `settings_page/keymap_presets.rs` uses `app::state::hit` and draws (`draw_browse`). This is against the dependency direction. Fix:
  - Move `BhRow` and `LookId` into `settings_page`.
  - Move `draw_browse` into `ui/page_bodies/settings/`.
  - Give it the hit types as arguments.
- **N1: the `Preview` type in two folders.** The state is in `preview_term/`. A second `impl Preview` for the drawing is in `ui/desktop_preview.rs`. So 7 fields are `pub(crate)`. The terminal-independence refactor handles this item later.
- **Ghostty-specific places** (for the terminal-independence refactor):
  - `preview_term/pane.rs` (`TermColors`, colours as Ghostty sets them)
  - `preview_term/mod.rs` (`TERM`, `COLORTERM`)
  - `main.rs` (kitty keyboard and graphics protocols)
  - `app/mod.rs` (OSC 22 pointer, OSC 52 clipboard)
  - `keymap/mod.rs` (a comment, and the kitty-protocol key names)

  `system/switch.rs` and `fonts/presets.rs` name Ghostty only in comments.
- **themes-switch can half-switch** (finding 17). If the palette of a theme is missing, the script applies the fonts first and then fails.
- **The install folder is also the previews folder** (finding 29, partly fixed). With the default `PREFIX`, `~/.local/share/theme-studio` holds the installed files and the preview fonts of the studio. The uninstall removes only its own files. A better way: keep the previews in another folder.
- **Open findings** in docs/refactor/PLAN.md section 4:
  - 16 (the test `real_configs_parse` in `system/desktop.rs` never runs)
  - 18 and 20 (XDG differences between the scripts and the studio)
  - 19 (no test compares `paths.sh` with `paths.rs`)
  - 25 (`draft_actions.rs` `n.max(1)`)
  - 26 (the cursor on Gap rows)
- `docs/plan.md` and `docs/migration.md` describe the old setup. Archive them or update them.
- `studio/src/main.rs` `--init-config` has no test.
- Duplicate code is at 1.24% (lizard). It comes from many small, similar draw helpers.
