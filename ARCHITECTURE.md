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
2. `studio/src/app/mod.rs`: the `App` struct (all state of the studio) and
   `App::run`, the loop. The `//!` comment at the top lists every file of the
   `app/` folder.
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
| 2 | `app/mod.rs`, `App::run` and `handle_event` | The loop waits up to 25 ms for a terminal event. A key press goes to `on_key`. |
| 3 | `app/input/actions.rs`, `on_key` and `handle_key` | Checks the special cases first (notice box, pop-ups, text fields). Then it turns the event into a `Key` (`keymap/key.rs`). |
| 4 | `app/input/focus.rs`, `contexts()` | With the picker focused it returns `[Picker, Colours, Global]`: the places to look for a binding, most specific first. |
| 5 | `keymap/mod.rs`, `Keymap::lookup` | Finds `l` in the `Picker` context. The result is `Lookup::Run(Action::Right)`. (`Lookup::Pending` means: wait for more keys, for example after the leader key.) |
| 6 | `app/input/actions.rs`, `run_action` | `Action::Right.group()` is `Column` (the row of `Right` in `keymap/table.rs`). On the Main page, with the colours side focused, the action goes to `run_colours`. |
| 7 | `app/input/actions.rs`, `run_colours` and `nudge` | The picker has the focus, so `nudge_step` makes a small step to the right. On the square that means: more saturation. It calls `set_hsv`. |
| 8 | `app/state/palette_editor.rs`, `PaletteEditor::set_hsv` and `edit` | Saves an undo step (one step for a run of changes to one key), then sets the colour in the palette (`model/palette.rs`). |
| 9 | `app/input/actions.rs`, `on_key` (end) | `sync_palette_preview` and `end_session_outside_picker` run. The loop marks the screen to be drawn again. |
| 10 | `app/mod.rs`, `refresh_before_draw` | `preview.sync_draft(&editor.palette)` (`preview_term/mod.rs`) writes the draft colours to a file that the hidden nvim in the preview reads. |
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
| 4 | `theme_apply_saved` and `fonts/presets.rs`, `apply_quietly` | Runs `font-switch <preset>` with `NO_RESTART=1` (write the font files, do not reload). |
| 5 | `model/store.rs`, `Store::apply` | Runs `theme-switch <palette>` and waits. The command comes from `model/paths.rs` `switch_command`: the command on `PATH`, or else `bash scripts/theme-switch.sh` next to the share folder. A non-zero exit becomes an error message with the script's stderr. |
| 6 | `scripts/theme-switch.sh` | Checks that the palette exists, then calls `write_theme_outputs`. Writes `current`. Unless `NO_RESTART` is set, calls `reload_apps theme`. |
| 7 | `scripts/lib/integrations.sh`, `write_theme_outputs` | Reads `share/integrations.json`. For each output with `"switch": "theme"`, it runs the generator from `share/integrations/` and writes the file. |
| 8 | `scripts/lib/integrations.sh`, `reload_apps` | For each app that has a theme output and a `reload` command, it runs the command, in list order: Ghostty, i3, Pywalfox. This is the only place that runs reload commands. |

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
| `app/` | `App`, the state of the studio, and everything that changes it. | `mod.rs` (struct, loop), `state/` (data: `nav.rs`, `palette_editor.rs`, `layout.rs`, `messages.rs`, `hit.rs`, `steps.rs`), `input/` (`actions.rs`, `focus.rs`, `mouse.rs`, `overlay.rs`, `colours.rs`), `pages/` (`settings/`, `fonts/`, `themes/`: the `impl App` code of each page), `persist.rs` (save, open, unsaved-changes prompt) |
| `ui/` | Drawing only. It reads `App` and draws a frame. | `mod.rs` (`draw`), `pages.rs`, `panes/`, `page_bodies/`, `overlays/`, `status/`, `layout.rs`, `desktop_preview.rs`, `widgets.rs` |
| `model/` | Data and rules, no screen code. | `palette.rs`, `color.rs`, `theme.rs`, `store.rs`, `paths.rs`, `history.rs`, `samples.rs`, `tournament.rs`, `settle.rs`, `encoding.rs` |
| `system/` | Talks to the outside: other programs, the network, the terminal. | `download/` (curl, tar, fc-cache), `prefetch.rs`, `desktop.rs` (reads i3, rofi and i3blocks configs), `out.rs` (terminal output) |
| `fonts/` | The fonts feature: presets, installed fonts, the sample image, the Download page and Tournament state. | `presets.rs`, `installed.rs`, `render.rs`, `download_page.rs`, `preview_page.rs`, `tournament_page.rs`, `types.rs` |
| `keymap/` | Keys, actions and bindings. | `key.rs`, `action.rs`, `table.rs`, `defaults.rs`, `mod.rs` (lookup, load, save) |
| `config/` | The studio's own settings files: `keymaps.json` and `settings.json`. | `mod.rs` (load, migrate), `settings.rs` (the settings structs) |
| `settings_page/` | State and data of the Settings page (F3). | `mod.rs`, `behaviour.rs`, `draft.rs`, `keymap_presets.rs`, `readme.rs` |
| `preview_term/` | The hidden terminals (nvim, a shell, yazi) behind the live preview. | `mod.rs` (`Preview`), `pane.rs` (`Pane`) |

Where the `impl App` code of the pages lives: `app/pages/settings/` (Settings page,
Behaviour tab, drafts, history), `app/pages/fonts/` (Download page, previews,
prefetch, Tournament) and `app/pages/themes/` (theme save, open and apply, and
`presets_actions.rs`, the methods of the keymap preset browser).

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
| `scripts/dev/` | Developer tools: `module-cycles.py` (finds dependency cycles), `lint-counts.sh`. |

### Dependency direction

```
ui  ->  app  ->  model
            ->  system
            ->  fonts, keymap, config, settings_page, preview_term
```

- `ui` reads `App`. `app` does not use `ui` in production code: `main.rs` passes
  `ui::draw` into `App::run`.
- `model` does not use `app`, `ui` or `system`. It starts one program:
  `Store::apply` runs `theme-switch`. It never uses the network.
- `system` uses `model` only.
- `fonts`, `keymap`, `config`, `settings_page` and `preview_term` do not use
  `ui`. `settings_page` reads two plain-data files of `app/state/`
  (`nav.rs` and `hit.rs`), and `model/theme.rs` uses `fonts/presets.rs` to
  migrate the font sizes.
- There are no cycles between the files. The flake check `cycles` runs
  `scripts/dev/module-cycles.py` and fails on a cycle.

---

## Key interfaces

**`App`** (`app/mod.rs`). One struct with all the state of the studio. Its
methods are spread over many files in `app/` (Rust allows more than one
`impl App` block). The state is grouped: `editor` (the palette, undo history,
picker numbers), `layout` (pane sizes), `messages`, `overlay` (the pop-up that
is open), `settings`, `keymap`, `fonts`, `settings_page`, `dl`, `hits`.
`App::run(terminal, draw)` is the loop.

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
font state folders, `switch_command`. No other file builds a user or system
path. When you change a path in one file, change it in the other too.

**`Store`** (`model/store.rs`). Holds the data, share and desktop folders.
Reads and writes `palettes.json` and runs `theme-switch` (`Store::apply`).

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
of a switch is the exit code of the last reload that ran, so a missing program
at the end of the list makes the switch exit with 127 although all files were
written (`docs/theme-studio.md`, "If an optional program is missing").

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
(no dependency cycles), `scripts` (golden tests) and `install` (install test). Any
failure fails the check.
Where: `flake.nix`, `scripts/dev/module-cycles.py`, `scripts/dev/lint-counts.sh`,
`docs/refactor/lint-baseline.txt`.

### Long-function markers
A function over about 30 lines gets a comment above it:
`// Long function (reason verified, phase N): <reason>.` It says why splitting would make
the code harder to read (for example, one `match` with one short arm for each case). The
words "reason verified" mean that a reviewer checked the reason.
Where: for example `app/input/actions.rs` (`handle_key`), `keymap/table.rs` (`ROWS`),
`settings_page/behaviour.rs` (`behaviour_rows`). Search for `Long function`.
