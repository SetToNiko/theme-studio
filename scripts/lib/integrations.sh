# shellcheck shell=bash
# Writes the app outputs that share/integrations.json lists.
# Source this after lib/paths.sh, then:
#
#   write_theme_outputs NAME DATA_FILE
#       writes the colour output of every app for palette NAME.
#       DATA_FILE is the palettes map (see json_data in lib/data.sh).
#
# integrations.json is the one list of apps. Each output has:
#   kind       how the file is made. Only "jq" exists now: the generator is a
#              jq program in share/integrations/ that gets the palettes map
#              and --arg n NAME. (Later kinds, such as templates and hooks,
#              get a new value here.)
#   dir        which state folder gets the file. Only "colors_dir" for now
#              (a function of paths.sh).
#   file       the file name in that folder
#   generator  the file in share/integrations/
# To add an app: add an entry to the list and put its generator file in
# share/integrations/. No change to this file.

# write_theme_outputs NAME DATA_FILE
write_theme_outputs() {
  local name=$1 data=$2 kind dir file generator folder
  while IFS=$'\t' read -r kind dir file generator; do
    case "$dir" in
      colors_dir) folder=$(colors_dir) ;;
      *) echo "integrations.json: unknown dir '$dir' for $file" >&2; return 1 ;;
    esac
    case "$kind" in
      jq)
        mkdir -p "$folder"
        jq -r --arg n "$name" -f "$(share_file "integrations/$generator")" \
          "$data" > "$folder/$file" ;;
      *) echo "integrations.json: unknown kind '$kind' for $file" >&2; return 1 ;;
    esac
  done < <(jq -r '.apps[].outputs[] | [.kind, .dir, .file, .generator] | @tsv' \
    "$(share_file integrations.json)")
}
