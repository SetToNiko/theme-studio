#!/usr/bin/env bash
# Switch font preset. A preset (fonts.json) has three fonts, each named after
# what it has to be:
#   monospace     a Nerd Font Mono: terminal (Ghostty, so also nvim/yazi),
#                 lock screen, "monospace"
#   proportional  rofi, GTK apps, "sans-serif"
#   icons         a Nerd Font Propo: i3 window titles and the bar (i3blocks);
#                 falls back to proportional if missing
# (Before fonts.json version 3 they were mono, ui and bar; both are read.)
# The sizes aren't the preset's but the theme's (themes.json `sizes`), so
# switching presets keeps them: $FONT_SIZES ("14 9 9", set by themes-switch
# and the studio), else the active theme's (~/.config/colors/theme), else
# the preset's own (fonts.json before version 4), else 14 11 11.
# Sizes get a per-host offset from ~/.config/host/fonts.json
# ({"monospace": 0, "proportional": 3, "icons": 3}; icons falls back to the
# proportional offset).
# Usage: font-switch.sh <font preset name>
here=$(dirname "$(readlink -f "$0")")
. "$here/lib/paths.sh" # data_file, share_file, colors_dir, fonts_dir, host_fonts_file
fonts="${FONTS:-$(data_file fonts.json)}"
out=$(fonts_dir)
# shellcheck disable=SC1091 # the other lib files are not followed either
. "$here/lib/data.sh" # json_data (either file layout)
# shellcheck disable=SC1091 # the lib files are found at run time, beside the script
. "$here/lib/integrations.sh" # write_font_outputs

name=${1:-}
[ -n "$name" ] || { echo "usage: font-switch.sh <font preset name>" >&2; exit 2; }

# Everything below reads the presets map (whichever layout the file has)
data=$(mktemp)
trap 'rm -f "$data"' EXIT
json_data "$fonts" > "$data"
jq -e --arg n "$name" 'has($n)' "$data" >/dev/null || { echo "no font preset '$name'" >&2; exit 1; }

# The sizes: as {"monospace": …, "proportional": …, "icons": …}, or null
sizes=null
if [ -n "$FONT_SIZES" ]; then
  read -r s_mono s_ui s_bar <<<"$FONT_SIZES"
  sizes=$(jq -n -c --arg a "$s_mono" --arg b "$s_ui" --arg c "$s_bar" \
    '{monospace: ($a | tonumber), proportional: ($b | tonumber), icons: ($c | tonumber)}') || sizes=null
else
  theme=$(cat "$(colors_dir)/theme" 2>/dev/null)
  themes=$(data_file themes.json)
  if [ -n "$theme" ] && [ -f "$themes" ]; then
    sizes=$(json_data "$themes" | jq -c --arg t "$theme" '.[$t].sizes // null') || sizes=null
  fi
fi

offsets=$(cat "$(host_fonts_file)" 2>/dev/null) || offsets='{}'
{ read -r mono; read -r mono_size; read -r ui; read -r ui_size; read -r bar; read -r bar_size; } < <(
  jq -r --arg n "$name" --argjson off "$offsets" --argjson sizes "${sizes:-null}" '
    def role($new; $old): .[$new] // .[$old];
    ($off | role("proportional"; "ui") // 0) as $ui_off
    | ($sizes // {}) as $s
    | .[$n] as $p
    | ($p | role("monospace"; "mono")) as $m
    | ($p | role("proportional"; "ui")) as $u
    | ($p | role("icons"; "bar") // $u) as $b
    | $m.family, ($s.monospace // $m.size // 14) + ($off | role("monospace"; "mono") // 0),
      $u.family, ($s.proportional // $u.size // 11) + $ui_off,
      $b.family, ($s.icons // $b.size // 11) + ($off | role("icons"; "bar") // $ui_off)' "$data")

for family in "$mono" "$ui" "$bar"; do
  fc-list -q ":family=$family" || echo "warning: font '$family' is not installed" >&2
done

mkdir -p "$out"
echo "$name" > "$out/current"

# Every app (Ghostty, i3, rofi, lock screen, GTK, fontconfig): written from the
# list in share/integrations.json.
# The generators get the three fonts as one JSON object.
fonts_json=$(mktemp)
trap 'rm -f "$data" "$fonts_json"' EXIT
jq -n --arg m "$mono" --arg ms "$mono_size" --arg u "$ui" --arg us "$ui_size" \
  --arg b "$bar" --arg bs "$bar_size" \
  '{monospace: {family: $m, size: $ms}, proportional: {family: $u, size: $us},
    icons: {family: $b, size: $bs}}' > "$fonts_json"
write_font_outputs "$fonts_json"

if [ -z "$NO_RESTART" ]; then
  pkill -USR2 -f '^ghostty( |$)'   # Ghostty reloads its config on SIGUSR2
  i3-msg restart >/dev/null        # titles and bar
fi
