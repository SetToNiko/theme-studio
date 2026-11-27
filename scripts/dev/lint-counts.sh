#!/usr/bin/env bash
# Lint ratchet: the number of warnings must never go up
# (docs/refactor/GOAL.md, criterion 7, and PRINCIPLES.md, project rule 3).
#
# Usage:
#   scripts/dev/lint-counts.sh                     measure clippy, shellcheck, the
#                                                  modules in dependency cycles,
#                                                  and the bad mod.rs files now
#                                                  (in the dev shell) and compare
#                                                  with the baseline
#   scripts/dev/lint-counts.sh compare NAME COUNT  compare one count with the
#                                                  baseline (`nix flake check`
#                                                  uses this)
#
# The baseline file has one line for each tool: "<name> <count>".
# LINT_BASELINE gives another path for it. When a count goes down, lower the
# number in the baseline file in the same commit. Then it can't go up again.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
project="$here/../.."
baseline_file="${LINT_BASELINE:-$project/docs/refactor/lint-baseline.txt}"

# Prints the allowed count for one tool, or nothing if the file has no line for it.
baseline_of() {
  awk -v name="$1" '$1 == name { print $2 }' "$baseline_file"
}

# Fails if COUNT is higher than the baseline of NAME.
compare() {
  local name=$1
  local count=$2
  local allowed
  allowed=$(baseline_of "$name")
  if ! [[ "$count" =~ ^[0-9]+$ ]]; then
    echo "$name: no count ($count)" >&2
    return 1
  fi
  if [ -z "$allowed" ]; then
    echo "lint-counts: no line for '$name' in $baseline_file" >&2
    return 1
  fi
  if [ "$count" -gt "$allowed" ]; then
    echo "$name: $count, baseline $allowed. It went up: fix the new ones." >&2
    return 1
  fi
  echo "$name: $count (baseline $allowed)"
  if [ "$count" -lt "$allowed" ]; then
    echo "  lower than the baseline: change the line to \"$name $count\" in $baseline_file"
  fi
}

# Prints the number of clippy warnings of the program (not counting the
# duplicates that the test build reports again).
clippy_count() {
  local output
  # A build error is not "fewer warnings": stop, so the ratchet cannot pass.
  if ! output=$(cd "$project/studio" && cargo clippy --all-targets 2>&1); then
    printf '%s\n' "$output" | grep -aE '^error' | head -5 >&2
    echo "lint-counts: clippy failed (build error), no count" >&2
    echo "build-error"
    return
  fi
  # The binary and, after the library target exists, the library each have
  # their own "generated N warnings" line. Add them.
  printf '%s\n' "$output" \
    | grep -aoE '\((bin|lib) "theme-studio"\) generated [0-9]+ warning' \
    | grep -oE '[0-9]+ warning' | grep -oE '[0-9]+' \
    | awk '{ total += $1 } END { print total + 0 }'
}

# Prints the number of mod.rs and lib.rs files with lines other than module
# declarations (scripts/dev/mod-layout.sh).
mod_layout_count() {
  bash "$here/mod-layout.sh" --count "$project/studio/src"
}

# Prints the number of shellcheck findings in the scripts.
shellcheck_count() {
  (shellcheck -f gcc "$project"/scripts/*.sh "$project"/scripts/lib/*.sh || true) | wc -l
}

# Prints the number of modules in dependency cycles (scripts/dev/module-cycles.py).
cycles_count() {
  python3 "$here/module-cycles.py" "$project/studio/src" | sed -n '1s/.*modules in cycles: //p'
}

if [ "${1:-}" = "compare" ]; then
  compare "$2" "$3"
  exit
fi

status=0
compare clippy "$(clippy_count)" || status=1
compare shellcheck "$(shellcheck_count)" || status=1
compare cycles "$(cycles_count)" || status=1
compare mod-layout "$(mod_layout_count)" || status=1
exit $status
