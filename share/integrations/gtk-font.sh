#!/usr/bin/env bash
# GTK 3 and 4 font settings, run by the "script" kind of
# scripts/lib/integrations.sh (fonts switch). It writes its own files:
# ~/.config/gtk-3.0/settings.ini and ~/.config/gtk-4.0/settings.ini.
# Usage: gtk-font.sh FONTS_FILE SHARE_DIR
#   FONTS_FILE  JSON object with the three fonts (see lib/integrations.sh)
#   SHARE_DIR   the share folder (it leads to scripts/lib/paths.sh)
fonts=$1
# paths.sh is beside the share folder, in the checkout and in the package.
# shellcheck disable=SC1091 # found at run time, relative to SHARE_DIR
here=$2/../scripts; . "$here/lib/paths.sh" # gtk_settings_file
{ read -r ui; read -r ui_size; } < <(jq -r '.proportional.family, .proportional.size' "$fonts")

# GTK 3 and 4: set gtk-font-name in [Settings], keeping any other settings.
# Apps pick it up when they next start.
for v in 3.0 4.0; do
  ini=$(gtk_settings_file "$v")
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
