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
#     wallpaper-rotate, fc-list, fc-match). Each stub writes its arguments to
#     calls.log. ghostty is not on PATH, on purpose: then the result does not
#     depend on the themes of an installed Ghostty.
# The result of a case is: the files under HOME, calls.log, stderr, and the exit
# code. They are compared with tests/scripts/golden/<case>/ with `diff -r`.
#
# Not covered: the rofi picker (no name given), and the login-screen part of
# font-switch.sh (it needs a writable /var/lib/ascii-boot, so it skips itself).
# Stub assumption: fc-list calls each family with "Mono" in its name
# monospaced (also "... Nerd Font Propo"). The golden files record the
# script under that assumption.
set -euo pipefail

# Safety: font-switch.sh copies a font into /var/lib/ascii-boot and rewrites
# the login screen's config when it can write there. The path is fixed in the
# script until plan step P1.3. On a machine where the folder is writable, the
# tests run in a private mount namespace with an empty READ-ONLY folder over
# it. Then the script skips the login-screen part, and the real folder stays
# unchanged. If that is not possible, the tests stop.
login_dir=/var/lib/ascii-boot
if [ -w "$login_dir" ]; then
  if [ -n "${IN_TEST_NAMESPACE:-}" ]; then
    echo "run.sh: $login_dir is still writable in the namespace. Stopped." >&2
    exit 1
  fi
  if ! unshare --user --map-root-user --mount true 2> /dev/null; then
    echo "run.sh: $login_dir is writable, and no private namespace is possible. Stopped," >&2
    echo "        so the tests do not change the real login screen." >&2
    exit 1
  fi
  exec env IN_TEST_NAMESPACE=1 unshare --user --map-root-user --mount \
    bash -c 'mount -t tmpfs -o ro tmpfs "$0" && exec "$@"' "$login_dir" "$0" "$@"
fi

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
make_stubs_dir() {
  mkdir -p "$work/stubs"
  local name
  for name in pkill i3-msg rofi pywalfox wallpaper-rotate fc-cache fc-match; do
    printf '#!/usr/bin/env bash\necho "%s $*" >> "$CALLS_LOG"\n' "$name" > "$work/stubs/$name"
  done
  # fc-list: every font is installed (-q succeeds). For a spacing question, a
  # family with "Mono" in its name is monospaced; other families are proportional.
  cat > "$work/stubs/fc-list" <<'EOF'
#!/usr/bin/env bash
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
    HOME="$home" "$setup"
  fi

  # Wait for background jobs too (theme-switch starts wallpaper-rotate with &):
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

# ---- Main -------------------------------------------------------------------

make_tools_dir
make_stubs_dir

all_cases() {
  cases_theme_switch
  cases_font_switch
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
