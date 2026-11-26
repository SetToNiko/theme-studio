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
# Usage: font-switch.sh <name>, or no argument for a rofi picker.
here=$(dirname "$(readlink -f "$0")")
. "$here/lib/paths.sh" # data_file, share_file
fonts="${FONTS:-$(data_file fonts.json)}"
out="$HOME/.config/fonts"
# shellcheck disable=SC1091 # the other lib files are not followed either
. "$here/lib/data.sh"        # json_data (either file layout)
. "$here/lib/rofi-picker.sh" # json_picker

# Menu rows: bold name, then each font's family written in that font, and
# "(active)" after the current preset
menu_rows='def esc: gsub("&"; "&amp;") | gsub("<"; "&lt;") | gsub(">"; "&gt;");
  def sample: "<span font_family=\"\(.family | esc)\">\(.family | esc)</span>";
  def role($new; $old): .[$new] // .[$old];
  to_entries[]
  | (.value | role("proportional"; "ui")) as $ui
  | "<b>\(.key | esc)</b>   monospace: \(.value | role("monospace"; "mono") | sample)   proportional: \($ui | sample)   icons: \(.value | role("icons"; "bar") // $ui | sample)"
    + (if .key == $cur then "   <i>(active)</i>" else "" end)'

if [ -n "$1" ]; then
  name=$1
else
  current=$(cat "$out/current" 2>/dev/null)
  # Delete messages use the active palette's red
  red=$(sed -n 's/^red=//p' "$HOME/.config/colors/colors.sh" 2>/dev/null)
  name=$(json_picker "$fonts" Fonts "$menu_rows" "$current" "${red:-#ff5555}" "Font preset") || exit 0
fi

[ -n "$name" ] || exit 0
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
  theme=$(cat "$HOME/.config/colors/theme" 2>/dev/null)
  themes=$(data_file themes.json)
  if [ -n "$theme" ] && [ -f "$themes" ]; then
    sizes=$(json_data "$themes" | jq -c --arg t "$theme" '.[$t].sizes // null') || sizes=null
  fi
fi

offsets=$(cat "$HOME/.config/host/fonts.json" 2>/dev/null) || offsets='{}'
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

# Ghostty (included from ghostty/config.ghostty). The empty value clears
# font-family's fallback list first.
printf 'font-family = ""\nfont-family = "%s"\nfont-size = %s\n' "$mono" "$mono_size" > "$out/ghostty"

# i3 window titles and the bar (i3blocks): the icons font
printf 'font pango:%s %s\n' "$bar" "$bar_size" > "$out/i3"

# rofi (imported from rofi/config.rasi)
printf 'configuration {\n    font: "%s %s";\n}\n' "$ui" "$ui_size" > "$out/rofi.rasi"

# Lock screen (sourced by lock/lock-screen.sh)
printf 'export XSECURELOCK_FONT=%q\n' "$mono:size=$mono_size" > "$out/lock.sh"

# GTK 3 and 4: set gtk-font-name in [Settings], keeping any other settings.
# Apps pick it up when they next start.
for v in 3.0 4.0; do
  ini="$HOME/.config/gtk-$v/settings.ini"
  if [ -L "$ini" ]; then
    echo "warning: $ini is a link (managed elsewhere?), not changing it" >&2
    continue
  fi
  mkdir -p "${ini%/*}"
  [ -f "$ini" ] || : > "$ini"
  tmp=$(mktemp)
  awk -v line="gtk-font-name=$ui $ui_size" '
    /^gtk-font-name=/ { next }
    { print }
    /^\[Settings\]/ && !done { print line; done = 1 }
    END { if (!done) { print "[Settings]"; print line } }' \
    "$ini" > "$tmp"
  cat "$tmp" > "$ini"
  rm -f "$tmp"
done

# fontconfig: point the generic families at the preset, for everything that
# asks for "monospace" or "sans-serif" (web pages, Electron apps, ...).
# sans-serif only follows the ui font when that font is proportional; a
# monospaced ui font would otherwise turn all sans-serif text monospace.
fc_dir="$HOME/.config/fontconfig/conf.d"
mkdir -p "$fc_dir"
# Monospaced fonts report a spacing (100, or 90 for dual-width); proportional
# ones report none. Checked before the file is rewritten, since fc-list reads it.
ui_is_mono=
fc-list ":family=$ui" spacing | grep -q 'spacing=' && ui_is_mono=1
alias_xml() {
  local family
  family=$(printf '%s' "$2" | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
  printf '  <alias binding="same">\n    <family>%s</family>\n    <prefer><family>%s</family></prefer>\n  </alias>\n' "$1" "$family"
}
tmp=$(mktemp)
{ echo '<?xml version="1.0"?>'
  echo '<!DOCTYPE fontconfig SYSTEM "fonts.dtd">'
  echo '<!-- Generated by font-switch.sh; edit fonts.json instead. -->'
  echo '<fontconfig>'
  alias_xml monospace "$mono"
  [ -z "$ui_is_mono" ] && alias_xml sans-serif "$ui"
  echo '</fontconfig>'; } > "$tmp"
cat "$tmp" > "$fc_dir/60-theme-fonts.conf"
rm -f "$tmp"

if [ -z "$NO_RESTART" ]; then
  pkill -USR2 -f '^ghostty( |$)'   # Ghostty reloads its config on SIGUSR2
  i3-msg restart >/dev/null        # titles and bar
fi
