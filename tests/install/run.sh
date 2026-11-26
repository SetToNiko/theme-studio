#!/usr/bin/env bash
# Install test: install into a temporary PREFIX, run the golden tests against
# the installed commands, uninstall, and check what is left.
#
# Usage:
#   tests/install/run.sh        (run it inside `nix develop`, or by `nix flake check`)
#
# Environment:
#   THEME_STUDIO_BIN   a prebuilt studio binary for install.sh (skips cargo).
#                      Without it, install.sh runs `cargo build`.
#   INSTALL_SH         the install script to test (default: ./install.sh of the
#                      project). A test of the test sets it to a broken copy.
#
# Safety: this script never touches your real HOME or PREFIX. It sets HOME,
# XDG_* and PREFIX inside a temporary folder first, and exits if they are not there.
set -euo pipefail

# ---- Safety: set every target to a temporary folder, before anything else ----
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_STATE_HOME="$HOME/.local/state"
export PREFIX="$tmp/prefix"
unset DESTDIR THEME_STUDIO_DATA THEME_STUDIO_SHARE THEME_STUDIO_SCRIPTS
case $HOME in "$tmp"/*) ;; *) echo "FAIL: HOME is not in the temporary folder"; exit 1 ;; esac
case $PREFIX in "$tmp"/*) ;; *) echo "FAIL: PREFIX is not in the temporary folder"; exit 1 ;; esac
mkdir -p "$HOME" "$PREFIX"

here=$(dirname "$(readlink -f "$0")")
project=$(readlink -f "$here/../..")
install_sh="${INSTALL_SH:-$project/install.sh}"
commands=(theme-studio theme-switch themes-switch font-switch)

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# ---- Install ----
keep="$PREFIX/share/theme-studio/previews/keep.txt"
mkdir -p "$(dirname "$keep")"
echo "user data" > "$keep"

bash "$install_sh" || fail "install.sh failed"

# The commands must resolve into PREFIX (not to a copy elsewhere on the PATH).
for name in "${commands[@]}"; do
  found=$(PATH="$PREFIX/bin:$PATH" command -v "$name" || true)
  [ "$found" = "$PREFIX/bin/$name" ] || fail "command -v $name gave '$found', expected $PREFIX/bin/$name"
done
echo "ok   the 4 commands resolve into PREFIX"

# ---- Golden tests against the installed commands, without THEME_STUDIO_SHARE ----
SCRIPTS_DIR="$PREFIX/bin" SCRIPTS_SUFFIX="" SCRIPTS_SHARE="" \
  bash "$project/tests/scripts/run.sh" || fail "golden tests failed with the installed commands"

# ---- Uninstall ----
bash "$install_sh" --uninstall || fail "uninstall failed"

for name in "${commands[@]}"; do
  if [ -e "$PREFIX/bin/$name" ] || [ -L "$PREFIX/bin/$name" ]; then
    fail "$PREFIX/bin/$name is still there after the uninstall"
  fi
done
[ ! -e "$PREFIX/share/theme-studio/scripts" ] || fail "share/theme-studio/scripts is still there after the uninstall"
[ ! -e "$PREFIX/share/theme-studio/share" ] || fail "share/theme-studio/share is still there after the uninstall"
[ -f "$keep" ] || fail "previews/keep.txt was removed by the uninstall (user data, finding 29)"
echo "ok   uninstall removed the commands and kept previews/keep.txt"
echo "install test passed"
