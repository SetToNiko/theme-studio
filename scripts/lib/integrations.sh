# shellcheck shell=bash
# Writes the app outputs that share/integrations.json lists.
# Source this after lib/paths.sh, then:
#
#   write_theme_outputs NAME DATA_FILE
#       writes the colour output of every app for palette NAME.
#       DATA_FILE is the palettes map (see json_data in lib/data.sh).
#
# integrations.json is the one list of apps. Each output has:
#   kind       how the file is made:
#                "jq"    the generator is a jq program in share/integrations/
#                        that gets the palettes map and --arg n NAME.
#                "hook"  the generator is a bash script in share/integrations/
#                        for outputs that need logic. The runner runs
#                        `bash <hook> NAME DATA_FILE SHARE_DIR`, and what the
#                        hook prints on stdout becomes the file.
#   dir        which state folder gets the file: "colors_dir" or "cache_dir"
#              (functions of paths.sh).
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
      cache_dir) folder=$(cache_dir) ;;
      *) echo "integrations.json: unknown dir '$dir' for $file" >&2; return 1 ;;
    esac
    case "$kind" in
      jq)
        mkdir -p "$folder"
        jq -r --arg n "$name" -f "$(share_file "integrations/$generator")" \
          "$data" > "$folder/$file" ;;
      hook)
        mkdir -p "$folder"
        bash "$(share_file "integrations/$generator")" "$name" "$data" \
          "$(dirname "$(share_file integrations.json)")" > "$folder/$file" ;;
      *) echo "integrations.json: unknown kind '$kind' for $file" >&2; return 1 ;;
    esac
  done < <(jq -r '.apps[].outputs[] | [.kind, .dir, .file, .generator] | @tsv' \
    "$(share_file integrations.json)")
}
