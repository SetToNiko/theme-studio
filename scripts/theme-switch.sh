#!/usr/bin/env bash
# Switch colour scheme.
# Usage: theme-switch.sh <palette name>
here=$(dirname "$(readlink -f "$0")")
. "$here/lib/paths.sh" # data_file, share_file, colors_dir, cache_dir
palettes="${PALETTES:-$(data_file palettes.json)}"
out=$(colors_dir)
# shellcheck disable=SC1091 # the other lib files are not followed either
. "$here/lib/data.sh" # json_data (either file layout)
# shellcheck disable=SC1091 # the lib files are found at run time, beside the script
. "$here/lib/integrations.sh" # write_theme_outputs, reload_apps

name=${1:-}
[ -n "$name" ] || { echo "usage: theme-switch.sh <palette name>" >&2; exit 2; }

# Everything below reads the palettes map (whichever layout the file has)
data=$(mktemp)
trap 'rm -f "$data"' EXIT
json_data "$palettes" > "$data"
jq -e --arg n "$name" 'has($n)' "$data" >/dev/null || { echo "no scheme '$name'" >&2; exit 1; }

mkdir -p "$out"
# The colour files (i3, rofi, colors.sh, Ghostty, Pywalfox) come from
# share/integrations.json
write_theme_outputs "$name" "$data"
echo "$name" > "$out/current"

if [ -z "$NO_RESTART" ]; then
  reload_apps theme
fi
