# shellcheck shell=bash
# Writes the app outputs that share/integrations.json lists.
# Source this after lib/paths.sh, then:
#
#   write_theme_outputs NAME DATA_FILE
#       writes the colour output of every app for palette NAME.
#       DATA_FILE is the palettes map (see json_data in lib/data.sh).
#   write_font_outputs FONTS_FILE
#       writes the font output of every app. FONTS_FILE is a JSON object
#       that font-switch.sh makes: {"monospace": {"family": "...", "size": "14"},
#       "proportional": {...}, "icons": {...}}.
#   reload_apps SWITCH
#       the one place that runs app reload commands. SWITCH is "theme" or
#       "fonts". It reloads each app that has an output for that switch and a
#       "reload" field, in list order. The scripts call it unless NO_RESTART is
#       set. Its exit status is the one of the last reload command.
#
# integrations.json is the one list of apps. Each output has:
#   switch     which switch writes it: "theme" (theme-switch.sh) or
#              "fonts" (font-switch.sh)
#   kind       how the file is made:
#                "jq"    the generator is a jq program in share/integrations/.
#                        A theme output gets the palettes map and --arg n NAME.
#                        A font output gets the FONTS_FILE object. Its program
#                        makes the whole text, with the final newline.
#                "hook"  the generator is a bash script in share/integrations/
#                        for outputs that need logic. What the hook prints on
#                        stdout becomes the file. The runner runs
#                          theme output: bash <hook> NAME DATA_FILE SHARE_DIR
#                          fonts output: bash <hook> FONTS_FILE SHARE_DIR
#                "script" (fonts outputs only) the generator is a bash script
#                        for apps that edit existing files in place (GTK). The
#                        runner runs `bash <script> FONTS_FILE SHARE_DIR`. The
#                        script writes its own files, and the runner writes
#                        nothing. It has no "dir" and no "file".
#   dir        which state folder gets the file: "colors_dir", "cache_dir",
#              "fonts_dir" or "fontconfig_dir" (functions of paths.sh).
#   file       the file name in that folder
#   generator  the file in share/integrations/
# An app that reloads after a switch also has (on the app, not on an output):
#   reload         the reload command as an argument array, for example
#                  ["i3-msg", "restart"]. No shell parses it.
#   reload_output  what happens to the command's output: "show" (default),
#                  "hide" (drop stdout) or "hide-all" (drop stdout and stderr)
#   note           why the command works (for people who read the list)
# The list order is the reload order: Ghostty, then i3, then Pywalfox.
# To add an app: add an entry to the list and put its generator file in
# share/integrations/. No change to this file.

# write_theme_outputs NAME DATA_FILE
write_theme_outputs() {
  local name=$1 data=$2 kind dir file generator folder
  while IFS=$'\t' read -r kind dir file generator; do
    folder=$(output_folder "$dir" "$file") || return 1
    case "$kind" in
      jq)
        mkdir -p "$folder"
        jq -r --arg n "$name" -f "$(share_file "integrations/$generator")" \
          "$data" > "$folder/$file" ;;
      hook)
        mkdir -p "$folder"
        run_hook_to_file "$folder/$file" bash "$(share_file "integrations/$generator")" \
          "$name" "$data" "$(dirname "$(share_file integrations.json)")" ;;
      *) echo "integrations.json: unknown kind '$kind' for $file" >&2; return 1 ;;
    esac
  done < <(jq -r '.apps[].outputs[] | select(.switch == "theme")
    | [.kind, .dir, .file, .generator] | @tsv' "$(share_file integrations.json)")
}

# write_font_outputs FONTS_FILE
write_font_outputs() {
  local fonts=$1 kind dir file generator folder share
  share=$(dirname "$(share_file integrations.json)")
  # A "script" output has no dir and no file: "-" fills the two columns, because
  # `read` would merge two tabs in a row.
  while IFS=$'\t' read -r kind dir file generator; do
    case "$kind" in
      jq)
        folder=$(output_folder "$dir" "$file") || return 1
        mkdir -p "$folder"
        # -j: no newline after the text (the generator ends it itself)
        jq -j -f "$(share_file "integrations/$generator")" \
          "$fonts" > "$folder/$file" ;;
      hook)
        folder=$(output_folder "$dir" "$file") || return 1
        mkdir -p "$folder"
        run_hook_to_file "$folder/$file" bash "$(share_file "integrations/$generator")" \
          "$fonts" "$share" ;;
      script)
        bash "$(share_file "integrations/$generator")" "$fonts" "$share" ;;
      *) echo "integrations.json: unknown kind '$kind' for $generator" >&2; return 1 ;;
    esac
  done < <(jq -r '.apps[].outputs[] | select(.switch == "fonts")
    | [.kind, .dir // "-", .file // "-", .generator] | @tsv' "$(share_file integrations.json)")
}

# reload_apps SWITCH
reload_apps() {
  local switch=$1 line mode
  local -a cmd
  while IFS= read -r line; do
    mode=$(jq -r '.[0]' <<<"$line")
    mapfile -t cmd < <(jq -r '.[1:][]' <<<"$line")
    case "$mode" in
      hide) "${cmd[@]}" >/dev/null ;;
      hide-all) "${cmd[@]}" >/dev/null 2>&1 ;;
      *) "${cmd[@]}" ;;
    esac
  done < <(jq -c --arg s "$switch" '.apps[]
    | select(.reload and any(.outputs[]; .switch == $s))
    | [.reload_output // "show"] + .reload' "$(share_file integrations.json)")
}

# run_hook_to_file TARGET COMMAND...: runs COMMAND and saves its stdout as TARGET.
# The output goes to a temp file first, then `cat` copies it into TARGET.
# Never write `COMMAND > TARGET` here: the shell would empty TARGET before the
# hook starts, and a hook (fontconfig) may ask a program that reads TARGET.
# Never `mv` the temp file over TARGET: a TARGET that is a link must stay a link.
run_hook_to_file() {
  local target=$1 tmp
  shift
  tmp=$(mktemp)
  "$@" > "$tmp"
  cat "$tmp" > "$target"
  rm -f "$tmp"
}

# output_folder DIR FILE: prints the state folder that the "dir" field names.
output_folder() {
  case "$1" in
    colors_dir) colors_dir ;;
    cache_dir) cache_dir ;;
    fonts_dir) fonts_dir ;;
    fontconfig_dir) fontconfig_dir ;;
    *) echo "integrations.json: unknown dir '$1' for $2" >&2; return 1 ;;
  esac
}
