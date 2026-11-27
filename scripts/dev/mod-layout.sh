#!/usr/bin/env bash
# Usage: scripts/dev/mod-layout.sh [--count] [DIR]
# Checks each mod.rs file and the lib.rs file under DIR (default: studio/src).
# Such a file holds only module declarations (the project-conventions skill,
# section "Language rules"):
#   - a `//!` header, empty lines, and comments
#   - declarations: `mod x;`, `pub mod x;`, `pub(crate) mod x;`,
#     `pub(super) mod x;`, or `pub(in <path>) mod x;`
#   - a `///` comment directly above each declaration
# Each other line is bad: `use`, code, `pub use`, tests, and attributes.
#
# Prints each bad line as "<path>:<line>: <reason>". With --count, it prints
# only the number of files with one or more bad lines.
# Exit status: 0. The gate compares the count with the ratchet baseline
# (scripts/dev/lint-counts.sh compare mod-layout COUNT).
set -euo pipefail

count_only=0
if [ "${1:-}" = "--count" ]; then
    count_only=1
    shift
fi
dir="${1:-studio/src}"
bad_files=0

while IFS= read -r -d '' file; do
    report=$(awk -v path="$file" '
        # A declaration, with an optional visibility.
        /^[[:space:]]*(pub(\((crate|super|in [^)]+)\))?[[:space:]]+)?mod[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*;[[:space:]]*$/ {
            if (previous !~ /^[[:space:]]*\/\/\//) {
                print path ":" NR ": declaration without a /// comment above it"
            }
            previous = $0
            next
        }

        # Empty lines and comments.
        /^[[:space:]]*$/ || /^[[:space:]]*\/\// {
            previous = $0
            next
        }

        {
            print path ":" NR ": not a declaration (move it into a child module): " $0
            previous = $0
        }
    ' "$file")
    if [ -n "$report" ]; then
        bad_files=$((bad_files + 1))
        if [ "$count_only" = 0 ]; then
            printf '%s\n' "$report"
        fi
    fi
done < <(find "$dir" \( -name mod.rs -o -path "$dir/lib.rs" \) -print0 | sort -z)

if [ "$count_only" = 1 ]; then
    echo "$bad_files"
fi
