#!/usr/bin/env bash
# Switch theme: a palette and a font preset together (themes.json, made in
# the theme studio). Fonts are applied first without restarting; the
# palette's switch then reloads Ghostty and restarts i3 once for both.
# Usage: themes-switch.sh <theme name>
here=$(dirname "$(readlink -f "$0")")
# shellcheck source=/dev/null
# (paths.sh is checked on its own; shellcheck does not follow sourced files unless run with -x.)
. "$here/lib/paths.sh" # data_file, share_file, colors_dir
themes="${THEMES:-$(data_file themes.json)}"
out=$(colors_dir)
# shellcheck disable=SC1091 # the other lib files are not followed either
. "$here/lib/data.sh" # json_data (either file layout)

name=${1:-}
[ -n "$name" ] || { echo "usage: themes-switch.sh <theme name>" >&2; exit 2; }

palette=$(json_data "$themes" | jq -r --arg n "$name" '.[$n].palette // empty')
fonts=$(json_data "$themes" | jq -r --arg n "$name" '.[$n].fonts // empty')
[ -n "$palette" ] && [ -n "$fonts" ] || { echo "no theme '$name'" >&2; exit 1; }

# The theme's font sizes (font-switch would otherwise take the active
# theme's, and this one only becomes active below)
sizes=$(json_data "$themes" | jq -r --arg n "$name" '.[$n].sizes | select(.) | "\(.monospace) \(.proportional) \(.icons)"')
FONT_SIZES="$sizes" NO_RESTART=1 bash "$here/font-switch.sh" "$fonts" || exit 1
bash "$here/theme-switch.sh" "$palette" || exit 1
mkdir -p "$out"
printf '%s\n' "$name" > "$out/theme"
