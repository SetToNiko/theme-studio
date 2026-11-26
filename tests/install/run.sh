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

# Guards: the binary must be a file, and the install script must be in the
# project or in the temporary folder (never run an unknown script).
if [ -n "${THEME_STUDIO_BIN:-}" ] && [ ! -f "$THEME_STUDIO_BIN" ]; then
  echo "FAIL: THEME_STUDIO_BIN ($THEME_STUDIO_BIN) is not a file" >&2
  exit 1
fi
[ -f "$install_sh" ] || { echo "FAIL: INSTALL_SH ($install_sh) is not a file" >&2; exit 1; }
install_real=$(readlink -f "$install_sh")
case $install_real in "$project"/*|"$tmp"/*) ;; *) echo "FAIL: INSTALL_SH ($install_real) is outside the project and the temporary folder" >&2; exit 1 ;; esac

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

# The desktop file exists and points at the installed binary.
desktop="$PREFIX/share/applications/theme-studio.desktop"
[ -f "$desktop" ] || fail "$desktop is missing after the install"
grep -qx "Exec=$PREFIX/bin/theme-studio" "$desktop" || fail "Exec= in the desktop file is not $PREFIX/bin/theme-studio"
echo "ok   the desktop file exists"

# The installed binary runs, and the share/ folder that it looks for exists.
# The studio has no flag that prints its share path, and its fallback (the
# project's share/) would hide a missing folder. So check the folder that
# model/paths.rs computes from the binary: exe/../../share/theme-studio/share.
"$PREFIX/bin/theme-studio" --help >/dev/null || fail "the installed theme-studio --help failed"
exe_dir=$(dirname "$(readlink -f "$PREFIX/bin/theme-studio")")
[ -d "$exe_dir/../share/theme-studio/share" ] || fail "the binary has no share/theme-studio/share folder beside its bin/"
echo "ok   the installed binary finds PREFIX/share/theme-studio/share"

# A second install over the first one works and leaves no stale file.
touch "$PREFIX/share/theme-studio/scripts/stale.sh" "$PREFIX/share/theme-studio/share/stale.json"
bash "$install_sh" >/dev/null || fail "the second install failed"
[ ! -e "$PREFIX/share/theme-studio/scripts/stale.sh" ] || fail "stale scripts/stale.sh is still there after the second install"
[ ! -e "$PREFIX/share/theme-studio/share/stale.json" ] || fail "stale share/stale.json is still there after the second install"
[ -f "$keep" ] || fail "previews/keep.txt was removed by the second install"
echo "ok   a second install leaves no stale files"

# A bad PREFIX or DESTDIR makes install.sh exit 2 and write nothing.
neg="$tmp/neg"
mkdir -p "$neg"
for bad in "relative/prefix" "$tmp/with space" "$tmp/a|b" "$tmp/a&b" "$tmp/a'b"; do
  rc=0
  (cd "$neg" && PREFIX="$bad" bash "$install_sh" >/dev/null 2>&1) || rc=$?
  [ "$rc" = 2 ] || fail "PREFIX='$bad' gave exit $rc, expected 2"
done
rc=0
(cd "$neg" && DESTDIR="relative" bash "$install_sh" >/dev/null 2>&1) || rc=$?
[ "$rc" = 2 ] || fail "a relative DESTDIR gave exit $rc, expected 2"
[ -z "$(find "$neg" -mindepth 1 -print -quit)" ] || fail "a rejected PREFIX or DESTDIR wrote files"
[ ! -e "$tmp/with space" ] && [ ! -e "$tmp/a|b" ] || fail "a rejected PREFIX created a folder"
echo "ok   a relative or unsafe PREFIX or DESTDIR exits 2 and writes nothing"

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
[ ! -e "$desktop" ] || fail "the desktop file is still there after the uninstall"
echo "ok   uninstall removed the commands and kept previews/keep.txt"
echo "install test passed"
