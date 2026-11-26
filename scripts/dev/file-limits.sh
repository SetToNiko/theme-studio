#!/usr/bin/env bash
# Usage: scripts/dev/file-limits.sh [DIR]
# Prints each .rs file under DIR (default: studio/src) that has more than 500
# production lines, as "<lines> <path>". Production lines are the lines before
# the "#[cfg(test)]" line that is followed by "mod tests". Files named tests.rs, and files in a tests/ folder,
# hold only tests and are skipped. Exit status: 1 if a file is over the limit,
# else 0. The limit is in project-conventions: 500 lines without tests.
set -euo pipefail

limit=500
dir="${1:-studio/src}"
over=0

while IFS= read -r -d '' file; do
    lines=$(awk '
        pending && /^[[:space:]]*mod tests/ { print pending - 1; found = 1; exit }
        { pending = 0 }
        /^#\[cfg\(test\)\]/ { pending = NR }
        END { if (!found) print NR }
    ' "$file")
    if [ "$lines" -gt "$limit" ]; then
        echo "$lines $file"
        over=1
    fi
done < <(find "$dir" -name '*.rs' ! -name tests.rs ! -path "*/tests/*" -print0 | sort -z)

exit "$over"
