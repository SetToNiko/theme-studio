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
tests/            golden-file tests of the scripts, the install test
docs/             theme-studio.md: the guide (also the studio's README tab)
                  plan.md: how it was built; migration.md: how it became a
                  project of its own, and what still ties it to one setup
ARCHITECTURE.md   how the code is built and how to extend it
CONTRIBUTING.md   the gates, the rules, and a map of the code for contributors
```

It works on NixOS today (packaged with Nix); making it work on other
distros is the next step (`docs/migration.md`, at the end).

New to the code? Read [ARCHITECTURE.md](ARCHITECTURE.md): how to read the
code, the modules, and how to add a program that Theme Studio configures.
Before you change the code, read [CONTRIBUTING.md](CONTRIBUTING.md): the checks
that must pass, the rules, and a map of the modules for parallel work.
To continue the open work on another machine, read
[docs/refactor/HANDOFF.md](docs/refactor/HANDOFF.md): the state, the next steps, and the setup.

## Refactor in progress

A refactor runs on the branch `refactor/kit-rules`. It is not merged into
`main` yet.

- **State on 2026-11-27:** Phase 3 (test locations). Waves 1 to 13 are
  merged. The next step is P3.11 (wave 14). It moves the last 9 tests out
  of `studio/src/app/tests/`. P3.9 and the Phase 3 review come after it.
- **Current state:** this note does not change with the refactor. The
  plan on the branch, `docs/refactor-fixes/PLAN.md`, shows the current
  state in its checked steps and its "Progress" table.

To resume the refactor:

```sh
git fetch origin
git switch refactor/kit-rules
git pull
claude    # start Claude Code in the project root
```

Then type `/refactor-kit` in the Claude Code session. The kit reads the
plan and selects the next action. The kit needs the skills `refactor-kit`
and `asd-ste100` in `~/.claude/skills/`. This repository does not
include them.

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

The Nix package brings the programs that it needs with it. Only the optional ones come from the system (see [Runtime dependencies](docs/theme-studio.md#runtime-dependencies)). Your palettes, font
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
| `curl`, `tar`, `xz`             | the font download page               |
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

## Development

Setup: install [Nix](https://nixos.org/) with flakes. `nix develop` opens a shell
with `cargo`, `clippy`, `rustfmt`, `shellcheck` and the other tools.

```sh
nix develop                      # the development shell (run it in the project folder)
cd studio && cargo test          # the Rust tests (inside nix develop)
cd studio && cargo run           # the studio, from the checkout
nix flake check                  # all checks: build and tests, rustfmt, clippy,
                                 # rustdoc, shellcheck, no module cycles,
                                 # script tests, install test
bash tests/scripts/run.sh        # golden-file tests of the switch scripts
bash tests/install/run.sh        # install into a temporary folder, test, uninstall
```

`bash tests/scripts/run.sh --update` rewrites the golden files. Use it only
when a change of the script output is wanted. `bash tests/install/run.sh`
runs `cargo build` unless `THEME_STUDIO_BIN` points to a built studio.

The code has these rules: no clippy warning (`-D warnings`), no shellcheck
finding, no dependency cycle between modules. `ARCHITECTURE.md` explains the
modules and the extension steps.

## Licence

MIT (`LICENSE`).
