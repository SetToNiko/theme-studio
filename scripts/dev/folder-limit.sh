#!/usr/bin/env bash
# Usage: scripts/dev/folder-limit.sh [DIR]
#        scripts/dev/folder-limit.sh --selftest
# Prints each folder under DIR (default: studio/src) that has more than 10
# modules, as "<modules> <path>".
#
# A module is one of these items in the folder:
#   - a .rs file. The files mod.rs, lib.rs, and main.rs do not count.
#   - a sub-folder with at least one .rs file at any depth.
# A folder "x/" next to "x.rs" is the same module as "x.rs": it counts once.
# A folder without .rs files is a data folder: it does not count.
#
# --selftest builds a sample tree and compares the result with the known one.
#
# Exit status: 1 if a folder is over the limit, else 0. The limit is in the
# project-conventions skill: 10 modules in one folder.
set -euo pipefail

limit=10

# Prints the number of modules in the folder $1.
count_modules() {
    local dir=$1 entry name
    local -A seen=()
    for entry in "$dir"/*; do
        name=$(basename "$entry")
        if [ -f "$entry" ] && [[ "$name" == *.rs ]]; then
            case "$name" in
                mod.rs | lib.rs | main.rs) continue ;;
            esac
            seen["${name%.rs}"]=1
        elif [ -d "$entry" ] && [ -n "$(find "$entry" -name '*.rs' -print -quit)" ]; then
            seen["$name"]=1
        fi
    done
    echo "${#seen[@]}"
}

# Prints "<modules> <path>" for each folder under $1 over the limit.
# Returns 1 if there is such a folder.
check_tree() {
    local root=$1 over=0 dir n
    while IFS= read -r -d '' dir; do
        n=$(count_modules "$dir")
        if [ "$n" -gt "$limit" ]; then
            echo "$n $dir"
            over=1
        fi
    done < <(find "$root" -type d -print0 | sort -z)
    return "$over"
}

if [ "${1:-}" = "--selftest" ]; then
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    # "full": 11 files and mod.rs, so 11 modules (over the limit).
    mkdir -p "$tmp/full"
    touch "$tmp/full/mod.rs"
    for i in 1 2 3 4 5 6 7 8 9 10 11; do touch "$tmp/full/f$i.rs"; done

    # "edge": 10 files, a folder next to a file (same module), and a data
    # folder. So 10 modules (at the limit, not over).
    mkdir -p "$tmp/edge/f1" "$tmp/edge/data"
    touch "$tmp/edge/mod.rs" "$tmp/edge/f1/inner.rs" "$tmp/edge/data/a.txt"
    for i in 1 2 3 4 5 6 7 8 9 10; do touch "$tmp/edge/f$i.rs"; done

    # "deep": 10 files and a folder with a .rs file two levels down. So 11.
    mkdir -p "$tmp/deep/sub/low"
    touch "$tmp/deep/sub/low/x.rs"
    for i in 1 2 3 4 5 6 7 8 9 10; do touch "$tmp/deep/f$i.rs"; done

    got=$(check_tree "$tmp" || true)
    want=$(printf '11 %s/deep\n11 %s/full' "$tmp" "$tmp")
    if [ "$got" != "$want" ]; then
        printf 'folder-limit self-test failed.\nExpected:\n%s\nGot:\n%s\nFix the count_modules function.\n' "$want" "$got" >&2
        exit 1
    fi
    echo "folder-limit self-test: ok (2 folders over the limit found)"
    exit 0
fi

check_tree "${1:-studio/src}"
