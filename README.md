# Theme Studio

Colour palettes and fonts for a Linux desktop, edited in a terminal with a
live preview of the desktop, and switched with one command. A **theme** is a
palette plus a font preset (and the fonts' sizes); switching one rewrites
the files each program reads (Ghostty, i3 and its bar, rofi, GTK,
fontconfig, nvim, the lock screen) and reloads what can be
reloaded.

```
studio/           the studio (Rust, ratatui)
scripts/          theme-switch, themes-switch, font-switch (bash, jq)
share/            files the studio and scripts read
packaging/nix/    the Nix package, a home-manager module
docs/             theme-studio.md: the guide (also the studio's README tab)
                  plan.md: how it was built; migration.md: how it became a
                  project of its own, and what still ties it to one setup
```

It works on NixOS today (packaged with Nix); making it work on other
distros is the next step (`docs/migration.md`, at the end).

## Install

There are two ways to install Theme Studio. Both give you the commands
`theme-studio`, `theme-switch`, `themes-switch` and `font-switch`.

### With Nix

```nix
inputs.theme-studio = {
  url = "github:SetToNiko/theme-studio";
  inputs.nixpkgs.follows = "nixpkgs";
};
# home-manager:  imports = [ theme-studio.homeManagerModules.default ];
#                programs.theme-studio.enable = true;
```

The Nix package brings most of its programs with it. The studio itself still needs `magick`, `curl`, `tar` and fontconfig from the system (see [Runtime dependencies](docs/theme-studio.md#runtime-dependencies)). Your palettes, font
presets and themes are not part of the project: they are read from
`$THEME_STUDIO_DATA`, by default `~/.config/theme-studio/`.

### Without Nix

```sh
./install.sh                      # installs into $HOME/.local
PREFIX=/usr/local ./install.sh    # installs into another prefix
./install.sh --uninstall          # removes what the script installed
```

The default `PREFIX` is `$HOME/.local`. Make sure `$PREFIX/bin` is in your
`PATH`.

`--uninstall` removes only what `install.sh` installed; your settings and the
downloaded previews stay.

To build, you need a Rust toolchain (`cargo`). To run, you need these
programs:

| Program                         | Needed for                           |
| ------------------------------- | ------------------------------------ |
| `bash`, `jq`                    | the switch commands, the studio      |
| `awk`, `sed`, `grep`, coreutils | the switch commands                  |
| fontconfig (`fc-list`, ...)     | fonts                                |
| ImageMagick (`magick`)          | font previews in the studio          |
| `curl`, `tar`                   | the font download page               |
| `ghostty`, `i3`, `pywalfox`     | optional: only the apps that you use |
| `nvim`, `yazi`, `git`, `xrdb`   | optional: preview panes, screen DPI  |

The full table is in
[`docs/theme-studio.md`](docs/theme-studio.md#runtime-dependencies).
It also says what happens when an optional program is missing.

## Build and test

```sh
nix build                                    # the package: studio + commands
cd studio && nix develop .. --command cargo test
```

## Licence

MIT (`LICENSE`).
