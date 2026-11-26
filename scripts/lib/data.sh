# shellcheck shell=bash
# Reads the preset files (palettes.json, fonts.json, themes.json).
# Source this file, then:
#
#   json_data FILE   prints the map of presets in FILE, one key per preset
#
# The files are either the plain map (version 1) or, from version 2,
# {"version": 2, "palettes" / "presets": { ... }}; json_data hides the
# difference.

# json_data FILE: prints the map of presets (either layout)
json_data() {
  jq 'if type == "object" and has("version") then (to_entries | map(select(.key != "version")) | .[0].value) else . end' "$1"
}
