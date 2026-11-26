#!/usr/bin/env bash
# Install Theme Studio without Nix.
#
# Usage:
#   ./install.sh               build and install into $HOME/.local
#   PREFIX=/usr/local ./install.sh
#   DESTDIR=/tmp/stage PREFIX=/usr ./install.sh   (for packagers: files go
#                              under $DESTDIR, but paths inside them use PREFIX)
#   ./install.sh --uninstall   remove what this script installed (same PREFIX)
#
# Layout (the same as the Nix package):
#   PREFIX/bin/theme-studio              the studio (a copy, not a link: it finds
#                                        PREFIX/share/theme-studio/share beside bin/)
#   PREFIX/bin/{theme-switch,themes-switch,font-switch}
#                                        links to the scripts; the scripts find
#                                        share/ beside their own folder
#   PREFIX/share/theme-studio/{scripts,share,docs/theme-studio.md}
#   PREFIX/share/applications/theme-studio.desktop
#
# It does not install the programs that Theme Studio runs (jq, ghostty, ...).
# It prints a warning for each one that it does not find.
# Needs: cargo (Rust toolchain). It builds with `cargo build --release --locked`.
# THEME_STUDIO_BIN=/path/to/theme-studio ./install.sh
#   installs that prebuilt binary and skips cargo (for tests and for packagers
#   that build the studio in another step, for example without network).
set -euo pipefail

PREFIX=${PREFIX:-$HOME/.local}
DESTDIR=${DESTDIR:-}
here=$(dirname "$(readlink -f "$0")")

# PREFIX goes into a sed command and into the desktop file (`Exec=`), and
# DESTDIR into paths. Allow only an absolute path of safe characters.
check_path() {
  local name=$1 value=$2
  if [[ ! $value =~ ^/[A-Za-z0-9._/+@:,=-]*$ ]]; then
    echo "install.sh: $name ('$value') must be an absolute path that uses only the characters A-Z a-z 0-9 . _ / + @ : , = - (no space, quote, | & or backslash). Set $name to such a path." >&2
    exit 2
  fi
}
check_path PREFIX "$PREFIX"
[ -z "$DESTDIR" ] || check_path DESTDIR "$DESTDIR"

root=$DESTDIR$PREFIX
bindir=$root/bin
sharedir=$root/share/theme-studio
appsdir=$root/share/applications
commands=(theme-switch themes-switch font-switch)

install_all() {
  local studio_bin=${THEME_STUDIO_BIN:-}
  if [ -n "$studio_bin" ]; then
    [ -x "$studio_bin" ] || { echo "install.sh: THEME_STUDIO_BIN ($studio_bin) is not an executable file." >&2; exit 1; }
  else
    if ! command -v cargo >/dev/null 2>&1; then
      echo "install.sh: cargo is not installed. Install the Rust toolchain (https://rustup.rs), then run this script again." >&2
      exit 1
    fi
    echo "Building the studio (cargo build --release --locked) ..."
    (cd "$here/studio" && cargo build --release --locked)
    studio_bin=$here/studio/target/release/theme-studio
  fi

  mkdir -p "$bindir" "$sharedir/docs" "$appsdir"
  rm -rf "$sharedir/scripts" "$sharedir/share"
  cp -r "$here/scripts" "$here/share" "$sharedir/"
  cp "$here/docs/theme-studio.md" "$sharedir/docs/"
  chmod +x "$sharedir"/scripts/*.sh
  echo "Installed $sharedir (scripts, share, docs)"

  install -m 755 "$studio_bin" "$bindir/theme-studio"
  echo "Installed $bindir/theme-studio"
  for name in "${commands[@]}"; do
    ln -sf "../share/theme-studio/scripts/$name.sh" "$bindir/$name"
    echo "Installed $bindir/$name"
  done

  sed "s|^Exec=.*|Exec=$PREFIX/bin/theme-studio|" "$here/packaging/theme-studio.desktop" \
    >"$appsdir/theme-studio.desktop"
  echo "Installed $appsdir/theme-studio.desktop"

  warn_missing
  case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Note: $PREFIX/bin is not on your PATH. Add it to run the commands by name." ;;
  esac
}

# Warn about each program that is missing. The two lists mirror the table
# "Runtime dependencies" in docs/theme-studio.md: change both together.
warn_missing() {
  local tool
  local required=(bash jq awk sed grep mktemp cat dirname readlink nice sha256sum kill
    fc-list fc-cache fc-scan magick curl tar)
  local -A optional=(
    [pkill]="the Ghostty reload" [ghostty]="Ghostty" [i3-msg]="the i3 reload"
    [pywalfox]="the Firefox reload" [nvim]="the preview pane" [yazi]="the preview pane"
    [git]="the preview shell" [ls]="the preview shell" [xrdb]="font previews (Xft.dpi)"
  )
  for tool in "${required[@]}"; do
    command -v "$tool" >/dev/null 2>&1 ||
      echo "Warning: '$tool' is not on PATH. Theme Studio needs it; install it with your package manager."
  done
  for tool in "${!optional[@]}"; do
    command -v "$tool" >/dev/null 2>&1 ||
      echo "Note: '$tool' is not on PATH (optional: only for ${optional[$tool]})."
  done
}

uninstall_all() {
  local name
  if [ -e "$bindir/theme-studio" ]; then
    rm -f "$bindir/theme-studio"
    echo "Removed $bindir/theme-studio"
  fi
  for name in "${commands[@]}"; do
    [ -e "$bindir/$name" ] || [ -L "$bindir/$name" ] || continue
    # A switch command is ours only if it links to our script.
    if [ "$(readlink "$bindir/$name" || true)" = "../share/theme-studio/scripts/$name.sh" ]; then
      rm -f "$bindir/$name"
      echo "Removed $bindir/$name"
    else
      echo "Left $bindir/$name (it is not a link that install.sh made)."
    fi
  done
  if [ -e "$appsdir/theme-studio.desktop" ]; then
    rm -f "$appsdir/theme-studio.desktop"
    echo "Removed $appsdir/theme-studio.desktop"
  fi
  # Only what install_all copied. Other files in share/theme-studio stay.
  if [ -d "$sharedir/scripts" ] || [ -d "$sharedir/share" ]; then
    rm -rf "$sharedir/scripts" "$sharedir/share" "$sharedir/docs/theme-studio.md"
    echo "Removed $sharedir (scripts, share, docs)"
  fi
  # Remove folders that are now empty (rmdir never removes a folder with files).
  rmdir "$sharedir/docs" "$sharedir" "$bindir" "$appsdir" "$root/share" 2>/dev/null || true
}

case "${1:-}" in
  "") install_all ;;
  --uninstall) uninstall_all ;;
  *)
    echo "Usage: ./install.sh [--uninstall]   (set PREFIX and DESTDIR in the environment)" >&2
    exit 2
    ;;
esac
