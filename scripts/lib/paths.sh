# Where the theme scripts find things (the studio's paths.rs does the same).
# Source this after setting $here to the script's own folder (resolved
# through links), then:
#
#   data_file NAME    your palettes.json, fonts.json or themes.json:
#                     $THEME_STUDIO_DATA/NAME, else
#                     ${XDG_CONFIG_HOME:-~/.config}/theme-studio/NAME
#   share_dir         the share folder: $THEME_STUDIO_SHARE, else ../share
#   share_file NAME   a file the scripts need (terminal-colors.json, …):
#                     $THEME_STUDIO_SHARE/NAME (set by the package), else
#                     ../share/NAME beside the scripts' folder
#   colors_dir        the colour state the switch scripts keep (~/.config/colors)
#   fonts_dir         the font state the switch scripts keep (~/.config/fonts)
#   host_fonts_file   this machine's font size offsets (~/.config/host/fonts.json)
#   cache_dir         the pywal-style colour cache (~/.cache/wal)
#   fontconfig_dir    the fontconfig drop-in folder (~/.config/fontconfig/conf.d)
#   gtk_settings_file VERSION   the GTK settings file, VERSION 3.0 or 4.0
#                     (~/.config/gtk-VERSION/settings.ini)
#   ghostty_themes_dir  your own Ghostty themes (~/.config/ghostty/themes)
#
# The first three of these five match colors_dir, fonts_dir and host_fonts_file
# in studio/src/paths.rs. If you change one side, change the other.

data_file() {
  printf '%s/%s\n' "${THEME_STUDIO_DATA:-${XDG_CONFIG_HOME:-$HOME/.config}/theme-studio}" "$1"
}

# The share folder: $THEME_STUDIO_SHARE (set by the package), else ../share
# beside the scripts' folder.
share_dir() {
  printf '%s\n' "${THEME_STUDIO_SHARE:-$here/../share}"
}

share_file() {
  printf '%s/%s\n' "$(share_dir)" "$1"
}

# The colour state folder: i3, colors.sh, ghostty and rofi.rasi files, plus
# `current` and `theme`.
colors_dir() {
  printf '%s\n' "$HOME/.config/colors"
}

# The font state folder: the generated font files and `current`.
fonts_dir() {
  printf '%s\n' "$HOME/.config/fonts"
}

# This machine's font size offsets (JSON), read by font-switch.sh.
host_fonts_file() {
  printf '%s\n' "$HOME/.config/host/fonts.json"
}

# The pywal-style cache folder that theme-switch.sh writes colors.json to.
cache_dir() {
  printf '%s\n' "$HOME/.cache/wal"
}

# The fontconfig drop-in folder: font-switch.sh's fontconfig output goes here.
fontconfig_dir() {
  printf '%s\n' "$HOME/.config/fontconfig/conf.d"
}

# The GTK settings file of one GTK version (VERSION is 3.0 or 4.0).
gtk_settings_file() {
  printf '%s\n' "$HOME/.config/gtk-$1/settings.ini"
}

# The user's own Ghostty themes folder (the Ghostty package has its own).
ghostty_themes_dir() {
  printf '%s\n' "$HOME/.config/ghostty/themes"
}
