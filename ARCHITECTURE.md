# Architecture

This file explains how Theme Studio is built. It is for a developer who knows
the basics of Rust and bash, and who does not know this project yet.

Theme Studio sets colours and themes (palettes and font presets) for the
system and for the programs that it is set up to configure. It has two parts:

- **The studio**: a Rust program that runs in a terminal (`studio/`). You edit
  a palette there and see a live preview.
- **The switch commands**: bash scripts (`scripts/`). They write the files that
  each program reads (Ghostty, i3, rofi, GTK, ...) and reload the programs.
  The studio runs them when you press Apply.

A word that you do not know? Look in [Concepts used in this codebase](#concepts-used-in-this-codebase).

Contents:
1. [How to read this codebase](#how-to-read-this-codebase)
2. [The modules](#the-modules)
3. [Key interfaces](#key-interfaces)
4. [How to extend](#how-to-extend)
5. [Concepts used in this codebase](#concepts-used-in-this-codebase)

---

## How to read this codebase

### Where to start

1. `studio/src/main.rs`: reads the command line, sets up the terminal, builds
   the `App` and starts the loop with `app.run(&mut terminal, ui::draw)`.
2. `studio/src/app/mod.rs`: the `App` struct (all state of the studio). The
   `//!` comment at the top lists every file of the `app/` folder. The loop,
   `App::run`, is in `studio/src/app/run.rs`.
3. `studio/src/app/input/actions.rs`: `run_action`, the one place that does
   what a key means.

Each folder has a `mod.rs` whose `//!` comment at the top says what the folder is
for. Read it before you open the files.

### Walk-through 1: a key press changes a colour

Situation: the Main page is open, the keyboard is in the colour picker (the
square), and you press `l` (the default Vim key for "right"). The colour of the
selected palette key gets a little more saturated, and you see it on screen.

| Step | File and function | What happens |
|---|---|---|
| 1 | `main.rs`, `app.run(&mut terminal, ui::draw)` | Starts the loop. `ui::draw` is passed in as a function, so `app` does not need to know the `ui` code. |
| 2 | `app/run.rs`, `App::run` and `handle_event` | The loop waits up to 25 ms for a terminal event. A key press goes to `on_key`. |
| 3 | `app/input/actions.rs`, `on_key` and `handle_key` | Checks the special cases first (notice box, pop-ups, text fields). Then it turns the event into a `Key` (`keymap/key.rs`). |
| 4 | `app/input/focus.rs`, `contexts()` | With the picker focused it returns `[Picker, Colours, Global]`: the places to look for a binding, most specific first. |
| 5 | `keymap/mod.rs`, `Keymap::lookup` | Finds `l` in the `Picker` context. The result is `Lookup::Run(Action::Right)`. (`Lookup::Pending` means: wait for more keys, for example after the leader key.) |
| 6 | `app/input/actions.rs`, `run_action` | `Action::Right.group()` is `Column` (the row of `Right` in `keymap/table.rs`). On the Main page, with the colours side focused, the action goes to `run_colours`. |
| 7 | `app/input/actions.rs`, `run_colours` and `nudge` | The picker has the focus, so `nudge_step` makes a small step to the right. On the square that means: more saturation. It calls `self.set_hsv(...)`, which is `App::set_hsv` in `app/mod.rs`. |
| 8 | `app/mod.rs`, `App::set_hsv`, then `app/state/palette_editor.rs`, `PaletteEditor::set_hsv` and `edit` | There are two `set_hsv`. `App::set_hsv` is a thin wrapper: it works out if the keyboard is in the picker and calls `self.editor.set_hsv(...)`, the `PaletteEditor` one. That one keeps the numbers in range, then calls `PaletteEditor::edit`, which saves an undo step (one step for a run of changes to one key) and sets the colour in the palette (`model/palette.rs`). |
| 9 | `app/input/actions.rs`, `on_key` (end) | `sync_palette_preview` (in `app/persist.rs`; it only does work while the palette list is open, so here it returns at once) and `end_session_outside_picker` run. `run` then marks the screen to be drawn again. |
| 10 | `app/run.rs`, `refresh_before_draw` | `preview.sync_draft(&editor.palette)` (`preview_term/mod.rs`) writes the draft colours to a file that the hidden nvim in the preview reads. |
| 11 | `ui/mod.rs`, `draw` | Draws the frame. First `reset_frame_state` empties the hit map. Then the panes draw the new colour: `ui/panes/colours.rs` (list), `ui/panes/picker.rs`, `ui/panes/preview.rs` and `ui/desktop_preview.rs` (preview). All of them read `app.editor.palette`. |

Nothing is written to `palettes.json` yet. The change lives in memory (and in
the draft colours file) until you save.

### Walk-through 2: Apply (a) to generated files and reload

`s` (Save) only writes the studio's own files (`palettes.json`, `fonts.json`,
`themes.json`). The files of the programs change when you press `a` (Apply).

| Step | File and function | What happens |
|---|---|---|
| 1 | `keymap/defaults.rs` and `app/input/actions.rs`, `run_action` | `a` is `Action::Apply`, group `Theme`. It goes to `run_theme_action`. |
| 2 | `app/pages/themes/theme_actions.rs`, `theme_apply` | If something is unsaved, it saves first (`theme_save(true)`), else it goes straight to `theme_apply_saved`. |
| 3 | `app/persist.rs` `save_as` and `model/store.rs` `Store::save` | A palette that changed is written to `palettes.json`. The old file goes to the history first (`model/history.rs`). |
| 4 | `theme_apply_saved` and `fonts/presets.rs`, `apply_quietly` | `apply_quietly` calls `system/switch.rs`, `apply_fonts(..., quiet = true)`. It runs `font-switch <preset>` with `NO_RESTART=1` (write the font files, do not reload). |
| 5 | `system/switch.rs`, `apply_theme` | Runs `theme-switch <palette>` and waits. `switch_command` (same file) chooses the command: the one on `PATH`, or else `bash scripts/theme-switch.sh` next to the share folder. A non-zero exit becomes an error message with the script's stderr. `system/switch.rs` is the only place in the studio that starts the switch scripts. |
| 6 | `scripts/theme-switch.sh` | Checks that the palette exists, then calls `write_theme_outputs`. Writes `current`. Unless `NO_RESTART` is set, calls `reload_apps theme`. |
| 7 | `scripts/lib/integrations.sh`, `write_theme_outputs` | Reads `share/integrations.json`. For each output with `"switch": "theme"`, it runs the generator from `share/integrations/` and writes the file. |
| 8 | `scripts/lib/integrations.sh`, `reload_apps` | For each app that has a theme output and a `reload` command, it runs the command, in list order: Ghostty, i3, Pywalfox. This is the only place that runs reload commands. If the program is not installed, the reload is skipped with a line on stderr. If a reload fails, a warning goes to stderr. In both cases the switch still exits with 0, because the files are written. |

You can also run step 6 by hand. This does not touch your real desktop: it
uses a temporary `HOME`, test data, and `NO_RESTART=1` (no reload):

```sh
tmp=$(mktemp -d)
HOME=$tmp NO_RESTART=1 THEME_STUDIO_DATA=$PWD/tests/scripts/fixtures/data \
  bash scripts/theme-switch.sh "CRT Amber"
ls "$tmp/.config/colors"
```

`themes-switch <theme>` (`scripts/themes-switch.sh`) does steps 4 to 8 for a
whole theme: `font-switch` without reload, then `theme-switch` with one reload.

---

## The modules

### The Rust code (`studio/src/`)

`main.rs` lists the folders. Each folder is a module with a `mod.rs`.

| Folder | Job | Main files |
|---|---|---|
| `app/` | `App`, the state of the studio, and everything that changes it. | `mod.rs` (struct), `run.rs` (loop), `tests/`, `state/` (data: `nav.rs`, `palette_editor.rs`, `layout.rs`, `messages.rs`, `hit.rs`, `steps.rs`), `input/` (`actions.rs`, `focus.rs`, `mouse.rs`, `overlay.rs`, `colours.rs`), `pages/` (`settings/`, `fonts/`, `themes/`: the `impl App` code of each page), `persist.rs` (save, open, unsaved-changes prompt) |
| `ui/` | Drawing only. It reads `App` and draws a frame. | `mod.rs` (`draw`), `pages.rs`, `panes/`, `page_bodies/`, `overlays/`, `status/`, `layout.rs`, `desktop_preview.rs`, `widgets.rs` |
| `model/` | Data and rules, no screen code. | `palette.rs`, `color.rs`, `theme.rs`, `store.rs`, `paths.rs`, `history.rs`, `samples.rs`, `tournament.rs`, `settle.rs`, `encoding.rs` |
| `system/` | Talks to the outside: other programs, the network, the terminal. | `switch.rs` (starts `theme-switch` and `font-switch`), `download/` (curl, tar, fc-cache), `prefetch.rs`, `desktop.rs` (reads i3, rofi and i3blocks configs), `out.rs` (terminal output) |
| `fonts/` | The fonts feature: presets, installed fonts, the sample image, the Download page and Tournament state. | `presets.rs`, `installed.rs`, `render.rs`, `download_page.rs`, `preview_page.rs`, `tournament_page.rs`, `types.rs`, `filters.rs`, `pick.rs`, `tree.rs` |
| `keymap/` | Keys, actions and bindings. | `key.rs`, `action.rs`, `table.rs`, `defaults.rs`, `mod.rs` (lookup, load, save) |
| `config/` | The studio's own settings files: `keymaps.json` and `settings.json`. | `mod.rs` (load, migrate), `settings.rs` (the settings structs) |
| `settings_page/` | State and data of the Settings page (F3). | `mod.rs`, `behaviour.rs`, `draft.rs`, `keymap_presets.rs`, `readme.rs`, `readme_layout.rs` |
| `preview_term/` | The hidden terminals (nvim, a shell, yazi) behind the live preview. | `mod.rs` (`Preview`), `pane.rs` (`Pane`) |

Where the `impl App` code of the pages lives: `app/pages/settings/` (Settings page,
Behaviour tab, drafts, history), `app/pages/fonts/` (Download page, previews,
prefetch, Tournament) and `app/pages/themes/` (theme save, open and apply, and
`presets_actions.rs`, the methods of the keymap preset browser).

#### Every file, with its job

Each line is the first line of the file's `//!` comment, in short. Paths are
relative to `studio/src/`. `mod.rs` files only list the files of their folder
(and, for `app/`, `model/` and `ui/page_bodies/`, say what the folder is for).

`main.rs`: reads the command line, sets up the terminal, starts the loop.

`app/`
- `mod.rs`: the `App` struct and its core methods; its `//!` lists all `app/` files.
- `run.rs`: the main loop: refresh, draw, poll, send each event to a handler.
- `persist.rs`: `App` methods that save, open and apply palettes and font presets, and the unsaved-changes prompt.
- `state/nav.rs`: where the user is: page, side, focus, open overlay, what is pending.
- `state/palette_editor.rs`: the palette being edited: colours, undo history, picker numbers.
- `state/layout.rs`: pane sizes, draggable borders, what is shown.
- `state/messages.rs`: the status-line message, the save confirmation box, the hover note.
- `state/hit.rs`: what is clickable where on screen (the hit map).
- `state/steps.rs`: index arithmetic for list cursors and scroll positions.
- `input/actions.rs`: key dispatch and the actions (`on_key`, `run_action`).
- `input/focus.rs`: which pane has the keyboard and how it moves.
- `input/mouse.rs`: the mouse.
- `input/overlay.rs`: the keys of the open pop-up.
- `input/colours.rs`: the colour list rows and the text input of the colour picker.
- `pages/settings/`: `settings_actions.rs` (the page as a whole), `settings_keymaps_actions.rs` (Keymaps tab), `settings_history_actions.rs` (History tab), `draft_actions.rs` (unsaved changes), `behaviour_actions.rs` (keys of the Behaviour tab), `behaviour_colours_actions.rs` (looks and colour rows), `behaviour_labels_actions.rs` (row texts).
- `pages/fonts/`: `fonts_dl_actions.rs` (rows, status, polling of the Download page), `fonts_install_actions.rs` (install and remove fonts), `fonts_preview_actions.rs` (the preview), `prefetch_actions.rs` (prefetching previews), `tour_actions.rs` (Tournament set-up), `tour_rounds.rs` (Tournament rounds and results).
- `pages/themes/`: `theme_actions.rs` (open, save, apply, the open list, sizes), `presets_actions.rs` (the keymap preset browser).
- `tests/`: tests of the app, one file per area: `font_downloads.rs`, `input.rs`, `layout.rs`, `lifecycle.rs`, `pages_fonts.rs`, `pages_settings.rs`, `palette.rs`; `mod.rs` has the shared set-up helpers.

`ui/`
- `mod.rs`: `draw`, the entry point of drawing.
- `pages.rs`: what fills the window for each page.
- `layout.rs`: where each pane goes for a window size.
- `pane_chrome.rs`: border colour and clickable title shared by the pages.
- `widgets.rs`: small drawing pieces and the cast helpers that several screens share.
- `desktop_preview.rs`: the live desktop preview: bar, rofi menu, two windows.
- `panes/`: `colours.rs` (Palette pane), `picker.rs` (Colour pane), `preview.rs` (Preview pane).
- `page_bodies/`: the bodies of pages and tabs. `fonts_column.rs` (Main page fonts column), `fonts_roles.rs` (role box), `fonts_download.rs` (Download page), `fonts_download_details.rs`, `fonts_download_preview.rs`, `tournament_tab.rs`, `rows.rs` (shared list helpers).
- `page_bodies/settings/`: the Settings page: `mod.rs` (tab strip), `keymaps.rs`, `behaviour.rs`, `history.rs`, `readme.rs`.
- `overlays/`: `mod.rs` (key hint and save notice), `dialogs.rs` (help, unsaved changes, save as, prompts, dropdowns), `help.rs` (the key list).
- `status/`: `mod.rs` (page tabs, message, key hints), `rows.rs` (which actions the hints show).

`model/`
- `palette.rs`: a palette, its 34 keys, the derive rules.
- `color.rs`: colour type and conversions (hex, HSV, HSL, contrast).
- `theme.rs`: a theme (palette name, font preset name, sizes), themes.json, and the `PresetSizes` trait.
- `store.rs`: reading and writing palettes.json and the other data files.
- `paths.rs`: where the data, share, config and state folders are.
- `history.rs`: version history of the config files.
- `samples.rs`, `samples/langs.rs`: preview texts for fonts and the language table of the code previews (`samples/code/` and `frankenstein-1.txt` hold the text).
- `tournament.rs`: the font tournament bracket and its saved state.
- `settle.rs`: images wait for a pane size to settle.
- `encoding.rs`: base64 encoding.

`system/`
- `switch.rs`: the only place that starts `theme-switch` and `font-switch`.
- `download/`: `mod.rs` (the `Source` type and messages), `catalogue.rs` (font lists), `install.rs` (install job), `list.rs` (installed-fonts list file), `net.rs` (curl calls), `preview.rs` (preview font cache).
- `prefetch.rs`: fetching font previews ahead of time.
- `desktop.rs`: what the preview's model desktop is made of, read from real configs.
- `out.rs`: terminal output.

`fonts/`
- `mod.rs`: the fonts column of the Main page.
- `presets.rs`: font presets and fonts.json (`FontStore`).
- `installed.rs`: installed font families from `fc-list`.
- `render.rs`: the sample image (Pango markup).
- `download_page.rs`, `preview_page.rs`, `tournament_page.rs`: state of the Download page, the preview, the Tournament tab.
- `types.rs`: plain data types and constants.
- `filters.rs`: filter chips of the font list.
- `pick.rs`: picking fonts for the roles.
- `tree.rs`: groups installed families into a two-level tree.

`keymap/`: `mod.rs` (keymaps, lookup), `key.rs` (key press in Vim notation), `action.rs` (every action), `table.rs` (the action table), `defaults.rs` (shipped presets).
`config/`: `mod.rs` (load and save keymaps.json and settings.json), `settings.rs` (settings structs).
`settings_page/`: `mod.rs` (the page), `behaviour.rs` (Behaviour tab rows), `draft.rs` (unsaved changes), `keymap_presets.rs` (preset browser), `readme.rs` (README tab), `readme_layout.rs` (word wrap and tables).
`preview_term/`: `mod.rs` (the hidden programs and their draft palette), `pane.rs` (one program in a hidden terminal).

### The script and data side

| Path | Job |
|---|---|
| `scripts/theme-switch.sh` | Switch the palette: write the colour files of all apps, reload. |
| `scripts/font-switch.sh` | Switch the font preset: write the font files of all apps, reload. |
| `scripts/themes-switch.sh` | Switch a theme (a palette and a font preset): `font-switch` without reload, then `theme-switch`. |
| `scripts/lib/paths.sh` | Where the scripts find things: data, share, and the output folders. The script side of `model/paths.rs`. |
| `scripts/lib/data.sh` | `json_data`: reads `palettes.json`, `fonts.json` or `themes.json` in either file layout. |
| `scripts/lib/integrations.sh` | The runner: writes the outputs and runs the reloads. |
| `share/integrations.json` | The list of apps: their outputs (file and generator) and their reload command. |
| `share/integrations/` | One small generator (`.jq` or `.sh`) for each output. |
| `share/terminal-colors.json`, `share/palette-derive.jq` | Which palette key is which terminal colour; the rules that derive palette colours. |
| `packaging/nix/` | The Nix package and the home-manager module. |
| `install.sh` | Install without Nix. |
| `tests/scripts/`, `tests/install/` | Golden-file tests of the switch scripts; the install test. |
| `scripts/dev/` | Developer tools: `module-cycles.py` (finds dependency cycles), `lint-counts.sh`, `file-limits.sh` (lists `.rs` files over 500 production lines). |

### Dependency direction

```
ui  ->  app  ->  model
            ->  system
            ->  fonts, keymap, config, settings_page, preview_term
```

- `ui` reads `App`. `app` does not use `ui` in production code: `main.rs` passes
  `ui::draw` into `App::run`.
- `model` imports nothing above it: not `app`, `ui`, `system`, `fonts`,
  `keymap`, `config`, `settings_page` or `preview_term`. It starts no program and
  never uses the network. `system/switch.rs` is the only place that starts
  `theme-switch` and `font-switch`. Where `model` needs something from `fonts`,
  it asks through a trait: `ThemeStore::migrate` takes `&dyn PresetSizes`
  (`model/theme.rs`) and `FontStore` implements it
  ([trait for a small boundary](#trait-for-a-small-boundary)).
- `system` uses `model` only.
- `fonts`, `keymap`, `config`, `settings_page` and `preview_term` do not use
  `ui`. `settings_page` reads two plain-data files of `app/state/`
  (`nav.rs` and `hit.rs`) in production code.
- There are no cycles between the files. The flake check `cycles` runs
  `scripts/dev/module-cycles.py` and fails on a cycle.

---

## Key interfaces

**`App`** (`app/mod.rs`). One struct with all the state of the studio. Its
methods are spread over many files in `app/` (Rust allows more than one
`impl App` block; see [split `impl App`](#split-impl-app)). The state is
grouped: `editor` (the palette, undo history,
picker numbers), `layout` (pane sizes), `messages`, `overlay` (the pop-up that
is open), `settings`, `keymap`, `fonts`, `settings_page`, `dl`, `hits`.
`App::run(terminal, draw)` is the loop (`app/run.rs`).

**`Action` and `ActionGroup`** (`keymap/action.rs`). `Action` lists everything
a key can do (`Quit`, `Save`, `Up`, `Right`, ...). `Action::group()` returns
one of four `ActionGroup` values (`Page`, `Window`, `Theme`, `Column`), and
`run_action` uses it to choose the method that handles the action.

**The action table** (`keymap/table.rs`). `ROWS` has one row for each action:
the action, its id in `keymaps.json`, a label for the help screen, and its group.

**`Keymap`** (`keymap/mod.rs`). A preset: a leader key and a list of
`Binding`s (context, key sequence, action). `Keymap::lookup(contexts, keys)`
answers `Run(action)`, `Pending` or `None`. The shipped presets are in
`keymap/defaults.rs`; the others (Helix, Emacs, VS Code, JetBrains) are made
from the Vim one.

**The hit map** (`app/state/hit.rs`). `app.hits` is a `HitMap`: a list of
screen areas that a click can hit (a `Target` each). The draw code calls
`hits.add(rect, target)` for every clickable thing. The mouse code calls
`hits.at(x, y)`.

**The integrations list** (`share/integrations.json`). A JSON list of apps.
Each app has `outputs` (`switch`, `kind`, `dir`, `file`, `generator`) and an
optional `reload` command. The field list is documented at the top of
`scripts/lib/integrations.sh`. The runner has three functions:
`write_theme_outputs`, `write_font_outputs` and `reload_apps`.

**The paths modules** (`studio/src/model/paths.rs` and `scripts/lib/paths.sh`).
The only places that know where files live: `home()`, the data folder
(`THEME_STUDIO_DATA`), the share folder (`THEME_STUDIO_SHARE`), the colour and
font state folders. No other file builds a user or system
path. When you change a path in one file, change it in the other too.

**`Store`** (`model/store.rs`). Holds the data, share and desktop folders.
Reads and writes `palettes.json`. It does not run any program.

**The switch functions** (`system/switch.rs`). `apply_theme(store, name)` runs
`theme-switch`; `apply_fonts(data, share, name, sizes, quiet)` runs
`font-switch`. `switch_command` there chooses the command: the one on `PATH`,
or else `bash scripts/<name>.sh` next to the share folder. In tests they never
start the scripts.

**`PresetSizes`** (`model/theme.rs`). A trait with one function,
`sizes_of(name)`, the base font sizes of a preset. `FontStore`
(`fonts/presets.rs`) implements it.

---

## How to extend

### 1. Add a program that Theme Studio configures

This is the main extension of the project. You do not change any script logic.
You add one entry to a list and one generator file. Example: a terminal
called `myterm` that gets a colour file. (`myterm` does not exist. It is only
the example. Copy the pattern of a real entry, for example `i3` or `ghostty`.)

**Step 1. Decide what the program needs.**
- Which switch changes its file: `"theme"` (a palette, `theme-switch`) or
  `"fonts"` (a font preset, `font-switch`)? A program can have outputs for both.
- Which file does the program read? Theme Studio writes the file. The program
  must read it. Most programs need one line in their own config for that
  (an `include` or `import` line). That line is the user's job: see the
  program's manual.
- Does the program need a reload? Which command?

**Step 2. Choose the folder (`dir`).** It must be one of these four. Each is a
function in `scripts/lib/paths.sh`:

| `dir` | Folder |
|---|---|
| `colors_dir` | `~/.config/colors` |
| `fonts_dir` | `~/.config/fonts` |
| `cache_dir` | `~/.cache/wal` |
| `fontconfig_dir` | `~/.config/fontconfig/conf.d` |

Need another folder? Add a function to `scripts/lib/paths.sh` and one line to
`output_folder` in `scripts/lib/integrations.sh`. This is the only reason to
change the runner.

**Step 3. Write the generator** in `share/integrations/`. Pick the kind:

- **`jq`** (the simplest). A jq program. A `theme` output gets the palettes map
  (`{"<palette name>": {"bg0": "#1a1005", "fg0": "...", ...}}`) as input and the
  palette name in `$n`. A `fonts` output gets `{"monospace": {"family", "size"},
  "proportional": {...}, "icons": {...}}`. The text that the program prints
  becomes the file. A `fonts` program must end its text with a newline itself
  (the runner uses `jq -j`). Compare `share/integrations/i3-colors.jq` (one line)
  and `share/integrations/ghostty-font.jq`. Our example,
  `share/integrations/myterm-colors.jq`:

  ```
  .[$n] | "background=\(.bg0)\nforeground=\(.fg0)\n"
  ```

- **`hook`**: a bash script, for logic that jq cannot express (an `if`, a check
  of an installed program). It prints the file text on stdout. The runner runs
  `bash <hook> NAME DATA_FILE SHARE_DIR` for a theme and
  `bash <hook> FONTS_FILE SHARE_DIR` for fonts. See
  `share/integrations/ghostty-colors.sh` and `fontconfig-font.sh`.

- **`script`** (`fonts` outputs only): a bash script that edits existing files
  itself (GTK, `share/integrations/gtk-font.sh`). It has no `dir` and no `file`.

**Step 4. Add the entry** to `share/integrations.json`, in the `apps` list:

```json
{
  "name": "myterm",
  "outputs": [
    { "kind": "jq", "switch": "theme", "dir": "colors_dir",
      "file": "myterm.conf", "generator": "myterm-colors.jq" }
  ]
}
```

**Step 5. Add the reload** (optional) on the app, not on an output:

```json
"reload": ["myterm", "--reload-config"],
"reload_output": "hide-all",
"note": "why this command works"
```

`reload` is an argument array (no shell parses it). `reload_output` is `show`
(the default), `hide` (drop stdout) or `hide-all` (drop stdout and stderr). The
reloads run once at the end of a switch, in list order, only for apps that have
an output for that switch, and only when `NO_RESTART` is not set. The exit code
of a switch is 0 even when the reload program is not installed (the runner skips
it and prints a line on stderr) or when a reload fails (it prints a warning on
stderr), because all files were written (`docs/theme-studio.md`, "If an optional
program is missing"). A wrong list or an unknown name still gives a non-zero code.

**Step 6. Try it** without touching your desktop (a temporary `HOME`, test data,
no reload):

```sh
tmp=$(mktemp -d)
HOME=$tmp NO_RESTART=1 THEME_STUDIO_DATA=$PWD/tests/scripts/fixtures/data \
  bash scripts/theme-switch.sh "CRT Amber"
cat "$tmp/.config/colors/myterm.conf"
```

Remove `NO_RESTART=1` only when you want to test the reload. It runs the real
command.

**Step 7. Update the golden tests.** `bash tests/scripts/run.sh` compares all
files that the scripts write with `tests/scripts/golden/`. Your new file makes the
test fail, which is expected. Read the diff, check that only your file and your
reload call are new, then run `bash tests/scripts/run.sh --update` and commit the
new golden files. If your app has a reload command, add the program's name to the
stub list in `make_stubs_dir` (`for name in pkill i3-msg pywalfox`) in
`tests/scripts/run.sh`, so that the test never starts the real program.

**Step 8. Document the program.** Add a row to the "Runtime dependencies" table
in `docs/theme-studio.md` (an optional program).

You do not change `install.sh` or `packaging/nix/package.nix`: both copy the
whole `share/` folder. The studio does not read `integrations.json`, so the
preview does not show a new program yet (`docs/refactor/PLAN.md`, section 7).

### 2. Add a key action

Example: a new action `Frobnicate` in the colours column.

1. `keymap/action.rs`: add `Frobnicate` to `enum Action`.
2. `keymap/action.rs`: the compiler stops at `has_table_row`. That `match` has no
   `_` arm on purpose. Add `Action::Frobnicate` to the list that returns `true`.
3. `keymap/table.rs`: add one row to `ROWS`:
   `row(A::Frobnicate, "frobnicate", "what it does", G::Column)`. The id is the
   name in `keymaps.json` and must be unique. The group decides where
   `run_action` sends it: `Page` (every page), `Window` (panes and windows of the
   Main page), `Theme` (save, apply, open, new) or `Column` (the focused column,
   or another page).
4. `app/input/actions.rs`: add an arm in the method of that group. For a colours
   action that is `run_colours`. (For the Settings page: `run_settings` in
   `app/pages/settings/settings_actions.rs`. For the Download page: `run_downloads`.)
5. `keymap/defaults.rs`: to give it a key, add `(C::Colours, A::Frobnicate, &["F"])`
   to the rows of `Keymap::vim()`. The other presets are made from Vim, so they get it
   too. Run the tests: they fail when two keys clash. Without a key, the action
   exists and the user can bind it on the Settings page, Keymaps tab.

Run `cargo test` in `studio/`. The tests in `keymap/action.rs` check that every
action has exactly one row and that the ids are unique.

### 3. Add a font download source

Example: a new place to download fonts from. A source gives `Entry` values (the
fonts in the list) and a way to install one.

1. `system/download/mod.rs`: add a variant to `enum Source`.
2. `system/download/catalogue.rs`: write a function that turns the source's
   list into `Entry` values (see `parse_nerd`), and call it in `load_catalogue`.
   Put the entries in `Catalogue.google` or `Catalogue.nerd`, or add a field
   (a new list needs a new tab in `fonts/download_page.rs`, `DlTab`).
3. `system/download/install.rs`: write `install_<source>` (see `install_nerd`:
   download, unpack, return the JSON that goes into the installed-fonts list) and
   add an arm in `install`.
4. Compile. The compiler lists each `match` on `Source` that needs a new arm
   (for example `detail_rows` in `ui/page_bodies/fonts_download_details.rs`).
   Check the `match` blocks with `_ =>` yourself: `list_name` and `dl_status` in
   `app/pages/fonts/fonts_dl_actions.rs`, and `fetch_preview_family` in
   `system/download/preview.rs`.
5. A new program for the download (like `curl` or `tar`) goes in the "Runtime
   dependencies" table in `docs/theme-studio.md`.

The work runs in a background thread and reports back with `notify`: see
[Background work](#background-work-with-threads-and-channels).

### 4. Add a settings value

Example: a yes/no setting "Installed fonts open at the role's current font" is
`list_opens_to_current_font`. Follow it as a model. These steps are for a
Behaviour-tab row.

1. `config/settings.rs`: add the field to its struct (here `FontSettings`) and to
   the struct's `Default` impl. The structs have `#[serde(default)]`, so an older
   `settings.json` without the field still loads, and gets the default.
2. `app/state/nav.rs`: add a variant to `enum BhRow` (here `ListOpensToCurrent`).
3. `settings_page/behaviour.rs`: add the row to `behaviour_rows()`, in the group
   where it shows.
4. `app/pages/settings/behaviour_labels_actions.rs`: add the variant to
   `behaviour_value` (the value text) and `behaviour_label` (the label). Both
   `match` blocks have no `_` arm, so the compiler tells you (it also tells you
   about any other `match` on `BhRow` that needs the new variant).
5. `app/pages/settings/behaviour_actions.rs`: add an arm that changes the value
   (here in `change_fonts_row`; a yes/no is `!value`, a list of choices uses
   `cycle`).
6. Use the value where it matters (here `app/mod.rs`, `build`).

The Behaviour tab edits a draft: nothing is written until the user presses Save
([The draft](#the-draft)).

---

## Concepts used in this codebase

Each entry: the name, what it is, and where it is.

### Data-driven integrations
The list of programs that Theme Studio configures is data (`share/integrations.json`),
not code. One runner (`scripts/lib/integrations.sh`) reads the list for every
switch. To add a program you add data and a generator file; you do not edit
the scripts.
Where: `share/integrations.json`, `share/integrations/`, `scripts/lib/integrations.sh`,
`scripts/theme-switch.sh`, `scripts/font-switch.sh`.

### The action table and the exhaustive check
Every key action has one row in a table: id, label and group. The functions
`id`, `describe` and `group` read the table. A `match` without a `_` arm
(`has_table_row`) makes the compiler refuse a new `Action` until you handle it
(an "exhaustive" `match`). A test checks the row exists.
Where: `keymap/table.rs`, `keymap/action.rs`.

### ActionGroup dispatch
`run_action` does not have one big `match` over all actions. It asks the action for
its group (`Page`, `Window`, `Theme`, `Column`) and calls the method of that group.
Each method has a smaller `match`.
Where: `keymap/action.rs` (`ActionGroup`, `Action::group`), `app/input/actions.rs`
(`run_action`).

### The hit map, rebuilt on every draw
The studio cannot ask the screen what is clickable. So each draw records the
clickable areas in `app.hits`. A mouse click looks up the last draw's list. The list
is emptied at the start of each draw.
Where: `app/state/hit.rs`, `ui/mod.rs` (`reset_frame_state`), `app/input/mouse.rs`.

### The draft
Changes on the Settings page (keymaps and behaviour) do not take effect and are not
written until Save. They live in a copy called the draft. `with_draft` swaps the draft
in as the live settings while an edit runs, then swaps it out. `Discard` drops it.
Where: `settings_page/draft.rs`, `app/pages/settings/draft_actions.rs`.

### Config migrations (versions)
Data files carry a `version`. When the studio starts, an older file is rewritten
in the current format (the old file goes to the history first). `keymaps.json`
migrates in `config/mod.rs`; `palettes.json`, `fonts.json` and `themes.json` migrate in
`model/store.rs` and `model/theme.rs`. `theme-studio --init-config` runs them without the UI.
Where: `config/mod.rs` (`KEYMAPS_VERSION`, `migrate`), `model/store.rs`
(`DATA_VERSION`, `FONTS_VERSION`, `migrate_data_file`), `model/theme.rs`
(`THEMES_VERSION`), `app/mod.rs` (`migrate_files`).

### Background work with threads and channels
Slow work (downloads, font previews, catalogue lists) runs in a thread (`std::thread::spawn`)
so the screen does not freeze. A thread sends its result into a channel
(`std::sync::mpsc`) with the helper `notify`. The main loop checks the channel on every
turn without waiting (`try_recv`) and updates the screen.
Where: `system/download/` (`Msg`, `notify`, `channel_pair`, `catalogue.rs`,
`install.rs`), `system/prefetch.rs`, `fonts/download_page.rs` (`try_recv`),
`app/pages/fonts/fonts_dl_actions.rs` (`poll_downloads`), `app/pages/fonts/prefetch_actions.rs`
(`poll_prefetch`), `fonts/render.rs`.

### Trait objects (`Box<dyn ...>`)
A `Box<dyn Write + Send>` is a value of any type that implements `Write`, chosen at run
time. The pseudo-terminal library hands out its parts in this form, and the studio
stores them like that.
Where: `preview_term/pane.rs` (`master`, `writer`, `child`, and the reader thread).

### `std::mem::replace`: take a value out for a while
`std::mem::replace(&mut x, new)` puts `new` into `x` and gives you the old value. The
studio uses it to take `app.overlay` out of `App` while it works, because the code
then needs `&mut App` at the same time. The overlay is put back afterwards.
Where: `ui/overlays/dialogs.rs` (`draw_overlay`), `app/input/overlay.rs`
(`on_overlay_key`), `app/state/palette_editor.rs` (undo and redo).

### Saturating cast helpers
Terminal sizes are `u16`, list lengths are `usize`, and a cursor step can be negative
(`i32`). Helper functions convert and stop at the largest (or smallest) value instead
of failing or wrapping. They replace `as` casts.
Where: `ui/widgets.rs` (`to_u16`, `to_u8`, `f64_to_u16`), `app/state/steps.rs`
(`to_i32`, `offset`, `wrapped`).

### `#[expect(..., reason = "...")]`
Clippy (the Rust linter) runs with `-D warnings`, so any warning fails the build.
When a warning is wrong for one place, `#[expect(lint, reason = "...")]` switches it off
there and records why. If the warning goes away later, `expect` itself warns, so
old switches do not stay.
Where: `ui/widgets.rs` (`f64_to_u16`), `app/mod.rs` (`App`), `app/pages/fonts/tour_actions.rs`,
`app/tests/`, and others (search for `#[expect(`).

### `pub(in crate::app)` visibility
Normal `pub` means every module. `pub(in crate::app)` means: only code inside the `app`
folder. It keeps helper methods of `App` private to the app code.
Where: `app/input/*.rs`, `app/pages/fonts/tour_rounds.rs`, `app/state/steps.rs`,
`app/state/nav.rs`.

### `include_str!` data files
`include_str!("file")` puts the text of a file into the program when it is compiled. The
sample code and text for the font previews are plain files, not Rust strings.
Where: `model/samples/langs.rs` (`code/*.rs`, `code/*.py`, ...), `model/samples.rs`
(`frankenstein-1.txt`).

### Golden-file tests
A golden-file test runs a program and compares everything it writes with a stored copy
(the "golden" files). `tests/scripts/run.sh` runs the switch scripts in a temporary
`HOME` with stub programs for the desktop (`pkill`, `i3-msg`, ...), and compares the
files, the calls to the stubs and the exit code with `tests/scripts/golden/`. If a
change is wanted, `--update` rewrites the golden files. `tests/install/run.sh` installs with
`install.sh` into a temporary folder, runs the same golden tests against the installed
commands, and uninstalls.
Where: `tests/scripts/`, `tests/install/`.

### Flake checks as gates
`nix flake check` runs: `build` (and the Rust tests), `fmt` (rustfmt), `clippy` (with
`-D warnings`), `doc` (rustdoc with `-D warnings`), `shellcheck` (all scripts), `cycles`
(no dependency cycles), `file-limits` (no `.rs` file over 500 production lines), `scripts`
(golden tests) and `install` (install test). Any failure fails the check.
Where: `flake.nix`, `scripts/dev/module-cycles.py`, `scripts/dev/lint-counts.sh`,
`scripts/dev/file-limits.sh`, `docs/refactor/lint-baseline.txt`.

### The file-limits gate
`scripts/dev/file-limits.sh` counts the production lines of each `.rs` file (the lines
before `#[cfg(test)]` and `mod tests`; files in a `tests/` folder and files named
`tests.rs` are skipped). It prints each file over 500 lines and exits with 1. The flake
check `file-limits` runs it on `studio/src`. A long file is split instead of allowed.
Where: `scripts/dev/file-limits.sh`, `flake.nix` (`file-limits`).

### Split `impl App`
`App` is one struct, but its methods are spread over many files in `app/`: each file has
its own `impl App { ... }` block (Rust allows this in any file of the same crate). So a
method is not always in the file of the struct. To find a method, search for its name:
`grep -rn "fn run_action" studio/src/app`. The `//!` at the top of `app/mod.rs` explains
the split and lists which file holds what.
Where: `app/mod.rs` (the `//!`), `app/run.rs`, `app/persist.rs`, `app/input/`, `app/pages/`.

### Let-chains (`if let ... && let ...`)
A let-chain joins `if let` tests and plain conditions with `&&` in one `if` (Rust 2024). It
avoids one `if` inside another. Without a chain you would nest them.
Where: `app/input/actions.rs` (`handle_key`: `if let Some(id) = self.preview.focus && matches!(...)`),
`config/mod.rs` (about line 214: `if write && found != Some(&full) && let Err(e) = write_json(...)`).
Search for `&& let ` to find more.

### chrono
`chrono` is a library for dates and times. The studio uses it for one thing: the local time
(`chrono::Local::now()`) in the one-line label of each saved version in the history.
Where: `model/history.rs` (the function with `chrono::Local::now()`), `studio/Cargo.toml`.

### Trait for a small boundary
A trait is a list of functions that a type promises to have. `PresetSizes` (one function,
`sizes_of`) says "give me the sizes of a font preset". `ThemeStore::migrate` asks for a
`&dyn PresetSizes`, and `FontStore` implements it, so `model/` does not import `fonts/` and
the dependencies keep one direction.
Where: `model/theme.rs` (`PresetSizes`, `ThemeStore::migrate`), `fonts/presets.rs`
(`impl PresetSizes for FontStore`), `app/mod.rs` (`migrate_files` passes `&self.fonts.store`).

### Long-function markers
A function over about 30 lines gets a comment above it:
`// Long function (reason verified, phase N): <reason>.` It says why splitting would make
the code harder to read (for example, one `match` with one short arm for each case). The
words "reason verified" mean that a reviewer checked the reason.
Where: for example `app/input/actions.rs` (`handle_key`), `keymap/table.rs` (`ROWS`),
`settings_page/behaviour.rs` (`behaviour_rows`). Search for `Long function`.
