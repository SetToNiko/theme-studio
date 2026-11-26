#!/usr/bin/env bash
# Lock screen font file (~/.config/fonts/lock.sh), written by the "hook" kind
# of scripts/lib/integrations.sh (fonts switch). lock/lock-screen.sh sources it.
# Usage: lock-font.sh FONTS_FILE SHARE_DIR
#   FONTS_FILE  JSON object with the three fonts (see lib/integrations.sh)
#   SHARE_DIR   the share folder (not used here)
# The file text goes to stdout. The runner saves it as the file.
fonts=$1
{ read -r mono; read -r mono_size; } < <(jq -r '.monospace.family, .monospace.size' "$fonts")

# %q quotes the value so that the shell can source it safely.
printf 'export XSECURELOCK_FONT=%q\n' "$mono:size=$mono_size"
