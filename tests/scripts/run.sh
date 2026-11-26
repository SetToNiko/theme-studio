#!/usr/bin/env bash
# Golden-file tests for the switch scripts (docs/refactor/GOAL.md, Verification 3).
#
# Usage:
#   tests/scripts/run.sh                 run all cases, compare with the golden files
#   tests/scripts/run.sh CASE...         run only these cases
#   tests/scripts/run.sh --update [CASE...]
#                                        write the golden files from the current
#                                        scripts. Do this only on purpose: the golden
#                                        files record the behavior that the refactor
#                                        must keep.
#
# Each case runs one script in a temporary HOME, with:
#   - THEME_STUDIO_DATA = a copy of tests/scripts/fixtures/data
#   - THEME_STUDIO_SHARE = the project's share/ (or $SCRIPTS_SHARE)
#   - a PATH with only the real tools that the scripts need (bash, jq, awk, ...)
#     and stub commands for the desktop programs (pkill, i3-msg, rofi, pywalfox,
#     fc-list, fc-match). Each stub writes its arguments to
#     calls.log. ghostty is not on PATH, on purpose: then the result does not
#     depend on the themes of an installed Ghostty.
# The result of a case is: the files under HOME, calls.log, stderr, and the exit
# code. They are compared with tests/scripts/golden/<case>/ with `diff -r`.
#
# Not covered: the rofi picker (no name given).
# Stub assumption: fc-list calls each family with "Mono" in its name
# monospaced (also "... Nerd Font Propo"). The golden files record the
# script under that assumption.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
project=$(readlink -f "$here/../..")
scripts="${SCRIPTS_DIR:-$project/scripts}"
share="${SCRIPTS_SHARE:-$project/share}"
golden_root="$here/golden"
fixtures="$here/fixtures"

update=no
if [ "${1:-}" = "--update" ]; then
  update=yes
  shift
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The real tools that the scripts use. Each one gets a link in $work/tools.
real_tools=(bash sh env jq awk gawk sed grep cat mkdir mktemp rm dirname readlink basename
  mv cp tr sort head tail wc cut date tee true false ls printf sleep)

make_tools_dir() {
  mkdir -p "$work/tools"
  local tool path
  for tool in "${real_tools[@]}"; do
    path=$(command -v "$tool" || true)
    case $path in
      /*) ln -sf "$path" "$work/tools/$tool" ;;
      *) ;; # a shell builtin, or not installed
    esac
  done
}

# Stub commands. Each one appends "<name> <args>" to $CALLS_LOG.
# The first line names bash by its full path: the Nix build sandbox has no
# /usr/bin/env.
make_stubs_dir() {
  mkdir -p "$work/stubs"
  local bash_path
  bash_path=$(command -v bash)
  local name
  for name in pkill i3-msg rofi pywalfox fc-cache fc-match; do
    printf '#!%s\necho "%s $*" >> "$CALLS_LOG"\n' "$bash_path" "$name" > "$work/stubs/$name"
  done
  # fc-list: every font is installed (-q succeeds). For a spacing question, a
  # family with "Mono" in its name is monospaced; other families are proportional.
  echo "#!$bash_path" > "$work/stubs/fc-list"
  cat >> "$work/stubs/fc-list" <<'EOF'
echo "fc-list $*" >> "$CALLS_LOG"
for arg in "$@"; do
  if [ "$arg" = "spacing" ]; then
    family=${1#:family=}
    case $family in *Mono*) echo "$family:spacing=100" ;; esac
  fi
done
exit 0
EOF
  chmod +x "$work/stubs"/*
}

# Runs one case. Arguments: case name, then the command line (env assignments
# first are allowed, as in "FONT_SIZES=16 font-switch.sh name").
# A function setup_<case> (with "-" changed to "_") runs first, if it exists.
# It gets HOME (the case's home folder) and DATA (the case's data folder).
run_case() {
  local name=$1
  shift
  local dir="$work/cases/$name"
  local home="$dir/home"
  mkdir -p "$home" "$dir/data"
  cp -r "$fixtures/data/." "$dir/data/"
  : > "$dir/calls.log"

  local setup="setup_${name//-/_}"
  if declare -F "$setup" > /dev/null; then
    HOME="$home" DATA="$dir/data" "$setup"
  fi

  # Wait for background jobs too (a script may start a command with &):
  # each process that the script starts inherits fd 9, the write end of the
  # pipe to `cat`. The pipeline ends only when the last of them has exited.
  # Without this, calls.log could miss a call that comes after the script ends.
  (
    status=0
    env -i HOME="$home" XDG_CONFIG_HOME="$home/.config" TMPDIR="$work" \
      PATH="$work/stubs:$work/tools" CALLS_LOG="$dir/calls.log" \
      THEME_STUDIO_DATA="$dir/data" THEME_STUDIO_SHARE="$share" \
      "$@" > "$dir/stdout" 2> "$dir/stderr" || status=$?
    echo "$status" > "$dir/status"
  ) 9>&1 | cat > /dev/null
  local status
  status=$(cat "$dir/status")

  # The result folder: the same paths on every machine.
  local result="$work/results/$name"
  mkdir -p "$result"
  cp -r "$home" "$result/home"
  echo "$status" > "$result/exit"
  sed "s|$home|<HOME>|g; s|$work|<TMP>|g" "$dir/stderr" > "$result/stderr"
  sed "s|$home|<HOME>|g; s|$work|<TMP>|g" "$dir/calls.log" > "$result/calls.log"
  sed "s|$home|<HOME>|g; s|$work|<TMP>|g" "$dir/stdout" > "$result/stdout"
  # git does not store empty folders, so the golden files have none.
  # Remove them from the result too, or a case that writes nothing fails.
  find "$result" -type d -empty -delete
}

# ---- Cases ------------------------------------------------------------------
# Each case: a name and the command. Add new cases here.

cases_theme_switch() {
  run_case theme-switch-palette bash "$scripts/theme-switch.sh" "CRT Amber"
  run_case theme-switch-no-restart env NO_RESTART=1 bash "$scripts/theme-switch.sh" "0x96f"
  run_case theme-switch-unknown bash "$scripts/theme-switch.sh" "No Such Palette"
}

# Setup functions: setup_<case> runs with HOME set to the case's home folder.

# The active theme gives the sizes (themes.json: CRT Amber, 14 9 9).
setup_font_switch_theme_sizes() {
  mkdir -p "$HOME/.config/colors"
  echo "CRT Amber" > "$HOME/.config/colors/theme"
}

# Per-host offsets. icons has no offset, so it takes the proportional one.
setup_font_switch_host_offsets() {
  mkdir -p "$HOME/.config/host"
  echo '{"monospace": 1, "proportional": 2}' > "$HOME/.config/host/fonts.json"
}

# GTK 3: other settings stay. GTK 4: a link is left alone, with a warning.
setup_font_switch_gtk_settings() {
  mkdir -p "$HOME/.config/gtk-3.0" "$HOME/.config/gtk-4.0" "$HOME/elsewhere"
  printf '[Settings]\ngtk-theme-name=Adwaita\ngtk-font-name=Old Font 10\ngtk-cursor-theme-size=24\n' \
    > "$HOME/.config/gtk-3.0/settings.ini"
  printf '[Settings]\ngtk-font-name=Managed 12\n' > "$HOME/elsewhere/settings.ini"
  ln -s ../../elsewhere/settings.ini "$HOME/.config/gtk-4.0/settings.ini"
}

cases_font_switch() {
  run_case font-switch-preset bash "$scripts/font-switch.sh" "JetBrains Mono"
  run_case font-switch-proportional bash "$scripts/font-switch.sh" "JetBrains + DejaVu Sans"
  run_case font-switch-sizes env FONT_SIZES="16 10 11" bash "$scripts/font-switch.sh" "JetBrains Mono"
  run_case font-switch-theme-sizes bash "$scripts/font-switch.sh" "JetBrains Mono"
  run_case font-switch-host-offsets bash "$scripts/font-switch.sh" "JetBrains + DejaVu Sans"
  run_case font-switch-gtk-settings bash "$scripts/font-switch.sh" "JetBrains Mono"
  run_case font-switch-no-restart env NO_RESTART=1 bash "$scripts/font-switch.sh" "JetBrains Mono"
  run_case font-switch-unknown bash "$scripts/font-switch.sh" "No Such Preset"
}

# A theme whose palette is not in palettes.json.
# Looks incorrect (recorded, not fixed: docs/refactor/PLAN.md, finding 17):
# themes-switch.sh applies the fonts first, then theme-switch.sh fails on the
# palette. The font files and fonts/current change, nothing reloads, and the
# exit code is 1. The desktop is left half-switched.
setup_themes_switch_missing_palette() {
  jq '.themes["Broken"] = {"palette": "No Such Palette", "fonts": "JetBrains Mono"}' \
    "$DATA/themes.json" > "$DATA/themes.json.new"
  mv "$DATA/themes.json.new" "$DATA/themes.json"
}

cases_themes_switch() {
  run_case themes-switch-theme bash "$scripts/themes-switch.sh" "CRT Amber"
  run_case themes-switch-missing-palette bash "$scripts/themes-switch.sh" "Broken"
  run_case themes-switch-unknown bash "$scripts/themes-switch.sh" "No Such Theme"
}

# ---- Main -------------------------------------------------------------------

make_tools_dir
make_stubs_dir

all_cases() {
  cases_theme_switch
  cases_font_switch
  cases_themes_switch
}

all_cases

wanted=("$@")
failed=0
checked=0
for result in "$work/results"/*; do
  name=$(basename "$result")
  if [ ${#wanted[@]} -gt 0 ] && [[ ! " ${wanted[*]} " == *" $name "* ]]; then
    continue
  fi
  golden="$golden_root/$name"
  if [ "$update" = yes ]; then
    rm -rf "$golden"
    mkdir -p "$golden_root"
    cp -r "$result" "$golden"
    echo "updated: $name"
    continue
  fi
  checked=$((checked + 1))
  if [ ! -d "$golden" ]; then
    echo "FAIL $name: no golden files (run with --update on the unchanged scripts)"
    failed=$((failed + 1))
  elif diff -r "$golden" "$result" > "$work/diff-$name"; then
    echo "ok   $name"
  else
    echo "FAIL $name:"
    sed 's/^/     /' "$work/diff-$name"
    failed=$((failed + 1))
  fi
done

if [ "$update" = no ]; then
  echo "$checked checked, $failed failed"
  [ "$failed" -eq 0 ]
fi
