# Theme Studio

Colour palettes and fonts for a Linux desktop, edited in a terminal with a
live preview of the desktop, and switched from rofi menus. A **theme** is a
palette plus a font preset (and the fonts' sizes); switching one rewrites
the files each program reads (Ghostty, i3 and its bar, rofi, GTK,
fontconfig, nvim, the lock and login screens) and reloads what can be
reloaded.

```
studio/           the studio (Rust, ratatui)
scripts/          theme-switch, themes-switch, font-switch (bash, jq, rofi)
share/            files the studio and scripts read
packaging/nix/    the Nix package, a home-manager module, a NixOS module
docs/             theme-studio.md: the guide (also the studio's README tab)
                  plan.md: how it was built; migration.md: how it became a
                  project of its own, and what still ties it to one setup
```

It works on NixOS today (packaged with Nix); making it work on other
distros is the next step (`docs/migration.md`, at the end).

## Build and test

```sh
nix build                                    # the package: studio + commands
cd studio && nix develop .. --command cargo test
```

## Use it from a NixOS flake

```nix
inputs.theme-studio = {
  url = "github:SetToNiko/theme-studio";
  inputs.nixpkgs.follows = "nixpkgs";
};
# home-manager:  imports = [ theme-studio.homeManagerModules.default ];
#                programs.theme-studio.enable = true;
# NixOS:         imports = [ theme-studio.nixosModules.default ];
```

Your palettes, font presets and themes aren't part of the project: they're
read from `$THEME_STUDIO_DATA`, by default `~/.config/theme-studio/`.

## Licence

MIT (`LICENSE`).
