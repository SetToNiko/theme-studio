#!/usr/bin/env bash
# Ghostty colour file (~/.config/colors/ghostty), written by the "hook" kind of
# scripts/lib/integrations.sh.
# Usage: ghostty-colors.sh NAME DATA_FILE SHARE_DIR
#   NAME       the palette name
#   DATA_FILE  the palettes map (JSON)
#   SHARE_DIR  the share folder (it holds terminal-colors.json)
# The Ghostty config lines go to stdout. The runner saves them as the file.
name=$1 data=$2 share=$3

# Ghostty: the palette sets background, foreground and all 16 ANSI colours,
# so programs using named terminal colours (starship, ls, git, yazi) match it.
# Which palette key goes where is in terminal-colors.json (shared with the
# theme studio's preview). A Ghostty theme of the same name, if there is one,
# still supplies the rest (cursor, selection). Palettes made by hand have
# none, and naming a missing theme is a Ghostty config error, so only name it
# when it exists, and take cursor and selection colours from the palette
# otherwise ("without_theme"). Without ghostty on PATH it can't be checked:
# keep the theme line.
theme_line=1
if ghostty_bin=$(command -v ghostty); then
  themes="$(dirname "$(readlink -f "$ghostty_bin")")/../share/ghostty/themes"
  [ -f "$themes/$name" ] || [ -f "$HOME/.config/ghostty/themes/$name" ] || theme_line=
fi
{ [ -n "$theme_line" ] && printf 'theme = %s\n' "$name"
  jq -r --arg n "$name" --arg theme "$theme_line" \
    --slurpfile map "$share/terminal-colors.json" '
    .[$n] as $p | $map[0] as $m
    | (if $theme == "" then $m.without_theme | to_entries[] | "\(.key) = \($p[.value])" else empty end),
      "background = \($p[$m.background])", "foreground = \($p[$m.foreground])",
      ($m.palette | to_entries[] | "palette = \(.key)=\($p[.value])")' \
    "$data"; }
