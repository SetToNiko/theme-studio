# Theme Studio

A terminal tool for making themes: colour palettes and fonts, edited with a
live preview of the desktop (i3 borders and bar, your real nvim, a shell or
yazi, a rofi menu), all drawn in the palette before it's saved. The
studio is in `studio/` (Rust, ratatui), the switch scripts in `scripts/`,
packaging in `packaging/`, these docs in `docs/`
(github.com/SetToNiko/theme-studio). How it came about is in `plan.md`; how
it became a project of its own, in `migration.md`.

## Themes, palettes and font presets

A **theme** is a name plus a palette plus a font preset, plus the fonts'
sizes. Palettes and font presets are building blocks: several themes can
share one, and editing it changes every theme that uses it. The sizes are
the theme's own, so swapping in another preset (or another font) keeps
them.

- **Palettes** (`palettes.json`) have 34 keys. 8 are base colours; the rest
  are derived from them (`palette-derive.jq`) until edited by hand, which
  pins them. Pins aren't stored: a key whose value differs from what the
  rules give counts as pinned.
- **Font presets** (`fonts.json`) have three fonts, each named after what it
  has to be:

  | Role         | Needs             | Used for                                                  |
  | ------------ | ----------------- | --------------------------------------------------------- |
  | Monospace    | a Nerd Font Mono  | terminal (so nvim, yazi), lock screen, "monospace" |
  | Proportional | proportional      | rofi, GTK apps, "sans-serif" (web pages, Electron)        |
  | Icons        | a Nerd Font Propo | i3 window titles and the bar                              |

  A preset only names the fonts. The **sizes**, one per role, belong to the
  theme (`themes.json`: `"sizes": {"monospace": 14, "proportional": 9,
  "icons": 9}`); `h` / `l` on a role in the studio change them, saved and
  applied a moment after the last key press. `font-switch.sh <name>` applies a
  preset at the active theme's sizes. Each machine can add an offset from
  `~/.config/host/fonts.json` (`{"monospace": 0, "proportional": 3,
  "icons": 3}`).

## Switching

| Command                   | What                                        |
| ------------------------- | ------------------------------------------- |
| `themes-switch.sh <name>` | a theme: palette and fonts together         |
| `theme-switch.sh <name>`  | a palette only, for a quick colour change   |
| `font-switch.sh <name>`   | a font preset only                          |

Each command needs a name. Without one it prints a usage message and exits
with status 2. The names are the ones in `palettes.json`, `fonts.json` and
`themes.json` in your data folder. A menu or key binding of your own can run
`theme-switch <name>`. The commands write the files the apps read
(`~/.config/colors/`, `~/.config/fonts/`, GTK and fontconfig settings) and
reload Ghostty and i3; rofi, GTK apps and the lock screen pick the change up
next time they start.

The studio itself (`$mod+Ctrl+t`) opens in a floating window.

## The studio

```sh
theme-studio                 # the active theme
theme-studio <name>          # a theme, or a palette
theme-studio --new           # a new theme
theme-studio --fonts         # start with the keyboard on the font roles
theme-studio --init-config   # write missing settings, update file formats, exit
```

Three pages, on `F1`–`F3`:

- **Main:** ¹ font roles, ² installed fonts, ³ font sample, ⁴ colour list,
  ⁵ colour picker (opens when you edit a colour), and the preview: ⁶ nvim
  and ⁷ the shell (`Y` for yazi, `R` for the rofi menu, `V` for a swatch
  view). `F12` or a click hands the keyboard to a preview window; `F12`
  again moves on and back.
- **Fonts:** download fonts from Google Fonts and Nerd Fonts (previewed
  before installing), the installed ones, and a **tournament**: pairs of
  fonts from a category, side by side, until one is left.
- **Settings:** Keymaps (rebind anything, presets), Behaviour (start-up
  defaults and looks), History (every config file's earlier versions,
  previewed against now before restoring) and README (this guide; `}` / `{`
  jump between headings).

`?` lists every key. Some of the Vim preset's:

| Key                        | What                                                  |
| -------------------------- | ----------------------------------------------------- |
| `s` / `S` / `a`            | save the theme / save as (forks edited parts) / apply |
| `o` / `n`                  | open / new theme                                      |
| `Space o p` / `Space o f`  | another palette / font preset for this theme          |
| `Space 1`–`Space 7`        | go to a pane                                          |
| `Enter` on a colour        | edit it in the picker (Enter keeps, Esc puts it back) |
| `u` / `Ctrl+r`             | undo / redo colour edits                              |
| `p`, `y`, `c`, `#`         | pin, copy hex, copy from another key, type a hex      |
| `Enter` on a font role     | pick from the fonts that fit it (applied at once)     |
| `r` on a font role         | the same family's face that fits, for one that doesn't |
| `F` on a font              | every role from its family                            |
| `Space r`                  | resize panes (or drag the borders)                    |
| `Space u`                  | undo the last config file change                      |
| `q`                        | quit (asks about unsaved changes)                     |

## Files

| Where                                   | What                                                   |
| --------------------------------------- | ------------------------------------------------------ |
| `~/.config/theme-studio/themes.json`    | themes                                                 |
| `~/.config/theme-studio/palettes.json`  | palettes                                               |
| `~/.config/theme-studio/fonts.json`     | font presets                                           |
| `~/.config/theme-studio/keymaps.json`, `settings.json` | the studio's keys and settings (the module's `dataDir` can link the folder to your dotfiles) |
| `share/terminal-colors.json`            | which palette key is each terminal colour              |
| the `fonts.installed-list` setting      | fonts the studio downloaded, installed by the home-manager module (`fontsList`) on the next rebuild |
| `~/.local/share/fonts/theme-studio/`    | downloaded fonts until that rebuild                    |
| `~/.local/share/theme-studio/previews/` | preview files of fonts that aren't installed           |
| `~/.local/state/theme-studio/`          | config history and tournaments                         |

The JSON files are written exactly as `jq .` would, so a save only changes
the lines that changed. Each has a `version`; the studio updates older
files when it starts, keeping the old one in the history.

## Runtime dependencies

The studio and the switch commands start the programs below. The column
"Package" says if the Nix package provides the program. Without Nix, you
install the programs yourself.

| Program | Used by | Need | Package |
| ------- | ------- | ---- | ------- |
| `bash` | the three switch commands; the studio (switch commands, bar blocks, preview shell) | required | yes |
| `jq` | the switch commands and all `.jq` generators | required | yes |
| `awk`, `sed`, `grep` | `gtk-font.sh`, `fontconfig-font.sh` | required | yes |
| `mktemp`, `cat`, `dirname`, `readlink`, `nice`, `sha256sum` (coreutils) | the switch commands; the studio (`nice`, `sha256sum` in the font download) | required | yes |
| `kill` | the studio (stops a bar block) | required | yes (procps) |
| `fc-list`, `fc-cache`, `fc-scan` (fontconfig) | the studio (font list, install, preview); `font-switch`, `fontconfig-font.sh` | required | yes |
| `magick` (ImageMagick) | the studio (font and text previews) | required | yes |
| `curl` | the studio (font download) | required | yes |
| `tar` | the studio (font download) | required | yes |
| `xz` | `tar` unpacks the Nerd Fonts archives (`.tar.xz`) | required | yes |
| `pkill` (procps) | the Ghostty reload in `share/integrations.json` | optional: Ghostty | yes |
| `ghostty` | `ghostty-colors.sh` (finds the built-in theme folder) | optional: Ghostty | yes |
| `i3-msg` (i3) | the i3 reload in `share/integrations.json` | optional: i3 | yes |
| `pywalfox` | the Firefox reload in `share/integrations.json` | optional: Pywalfox | no |
| `nvim` | the studio (preview pane) | optional | no |
| `yazi` | the studio (preview pane) | optional | no |
| `git`, `ls` | the demo commands in the preview shell | optional | no |
| `xrdb` | the studio (reads `Xft.dpi` for font previews) | optional | no |

Notes:

- The Nix package puts the programs of the "yes" rows in the `PATH` of the
  three switch commands and of the studio. The studio also gets ImageMagick,
  curl, tar and xz. Optional programs (`xrdb`, `nvim`, `yazi` and the desktop
  programs) come from the system.
- Each program that a `reload` entry of `share/integrations.json` runs is
  optional. If you add an app with a `reload` command, that program is
  optional too.
- The bar blocks in your `i3blocks/config` are your own commands. The studio
  runs them with `bash` and needs no other program for them.

### If an optional program is missing

A switch command writes all its files first and reloads the apps last. A
missing program never stops the files from being written. The reloads run
in the order of `share/integrations.json`, and a failed reload does not
stop the next one. The exit code of the switch command is the exit code of
the last reload that ran. For `theme-switch` that is Pywalfox: without
`pywalfox` it prints nothing (the output of that reload is hidden), writes
every file, and exits with status 127. For `font-switch` the last reload is
`i3-msg`: without i3 it prints "command not found" and exits with status
127. `themes-switch` stops when `theme-switch` fails, so in that case it does
not write `~/.config/colors/theme`. If Ghostty is not running, `pkill` exits
with status 1; this matters only when it is the last reload. A caller that
checks the exit code, for example a key binding, can see a failed switch
although the files are correct. In the studio, a missing `nvim` or `yazi`
shows an error for that pane only. A missing `xrdb` makes the font preview
use 96 DPI. A missing `magick` makes each preview render report an
ImageMagick error. A missing `jq`, `curl` or `nice` makes the switch or the
download fail with an error message.

## Building and testing

The package is `packaging/nix/package.nix` (`nix build` in the checkout);
a NixOS flake takes it as an input (see the README) and sets it up with a
module, which the project's flake exports:

- **home-manager**, `packaging/nix/home-module.nix`, `programs.theme-studio`:
  the studio and the three commands; `live` (a checkout: the commands run
  its scripts, so edits apply without a rebuild); `dataDir` (linked to
  `~/.config/theme-studio`); `fontsList` (downloaded fonts, installed);
  `~/.config/colors` and `~/.config/fonts` regenerated on every switch, with
  `fallbackPalette` / `fallbackFonts` when nothing is current.

To work on it:

```sh
cd studio
nix develop .. --command cargo test     # the tests
nix develop .. --command cargo run      # the studio, from the checkout
bash ../scripts/theme-switch.sh <name>  # a script, from the checkout
```

Tests that need the real data or a tool that isn't there skip themselves.
Environment variables:

- `THEME_STUDIO_DATA`: your palettes, font presets and themes (default
  `~/.config/theme-studio`).
- `THEME_STUDIO_SHARE`: the files the studio and scripts need (set by the
  package, or to the checkout's `share/` in live mode; a `cargo run` in the
  checkout finds it by itself).
- `THEME_STUDIO_SCRIPTS`: where the commands find their scripts (live mode
  sets it to the checkout's `scripts/`).
- `THEME_STUDIO_TIMING=1`: logs each font preview's render time to
  `~/.local/state/theme-studio/timing.log`.
