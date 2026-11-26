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
set -euo pipefail

PREFIX=${PREFIX:-$HOME/.local}
DESTDIR=${DESTDIR:-}
here=$(dirname "$(readlink -f "$0")")

root=$DESTDIR$PREFIX
bindir=$root/bin
sharedir=$root/share/theme-studio
appsdir=$root/share/applications
commands=(theme-switch themes-switch font-switch)

install_all() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "install.sh: cargo is not installed. Install the Rust toolchain (https://rustup.rs), then run this script again." >&2
    exit 1
  fi
  echo "Building the studio (cargo build --release --locked) ..."
  (cd "$here/studio" && cargo build --release --locked)

  mkdir -p "$bindir" "$sharedir/docs" "$appsdir"
  rm -rf "$sharedir/scripts" "$sharedir/share"
  cp -r "$here/scripts" "$here/share" "$sharedir/"
  cp "$here/docs/theme-studio.md" "$sharedir/docs/"
  chmod +x "$sharedir"/scripts/*.sh
  echo "Installed $sharedir (scripts, share, docs)"

  install -m 755 "$here/studio/target/release/theme-studio" "$bindir/theme-studio"
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

# Warn about each program that the scripts or the studio run and that is missing.
warn_missing() {
  local tool
  for tool in bash jq fc-list pkill sed awk grep ghostty i3-msg; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      echo "Warning: '$tool' is not on PATH. Theme Studio needs it; install it with your package manager."
    fi
  done
}

uninstall_all() {
  local name
  for name in theme-studio "${commands[@]}"; do
    if [ -e "$bindir/$name" ] || [ -L "$bindir/$name" ]; then
      rm -f "$bindir/$name"
      echo "Removed $bindir/$name"
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
