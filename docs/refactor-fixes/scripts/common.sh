#!/usr/bin/env bash
# Shared helpers for the wave scripts. The other scripts load this file with "source".
#
# A plan step is one line in the plan file. Example:
#   - [ ] P2.3 Extract Store: move file I/O into Store | Wave: 4 | Depends: P2.1 | Owns: src/store/, tests/store_test.rs | Target: SRP | Check: store tests
# The helpers below read the fields of such a line.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
source "$SCRIPT_DIR/config.sh"

# Make WORKTREE_ROOT an absolute path.
case "$WORKTREE_ROOT" in
  /*) ;;
  *) WORKTREE_ROOT="$PROJECT_ROOT/$WORKTREE_ROOT" ;;
esac
# Remove "/../" from the path. Example: /src/app/../app-worktrees -> /src/app-worktrees
WORKTREE_ROOT="$(cd "$(dirname "$WORKTREE_ROOT")" && pwd)/$(basename "$WORKTREE_ROOT")"

# shellcheck disable=SC2034 # wave-merge.sh writes its logs here.
LOG_DIR="$WORKTREE_ROOT/logs"

# Prints a message and stops the script.
die() {
  echo "ERROR: $*" >&2
  exit 1
}

# Prints the step ID with each dot escaped, for use in a regular expression.
# Example: P2.3 -> P2\.3
step_regex() {
  echo "${1//./\\.}"
}

# Prints the plan line of one step, or nothing if the step is not in the plan.
step_line() {
  local id
  id="$(step_regex "$1")"
  grep -m1 -E -- "^- \[[ x]\] $id " "$PROJECT_ROOT/$PLAN_FILE" || true
}

# Returns success if the plan marks the step as done ("- [x]").
step_is_done() {
  local id
  id="$(step_regex "$1")"
  grep -q -E -- "^- \[x\] $id " "$PROJECT_ROOT/$PLAN_FILE"
}

# Prints the value of one field of a step. Example: step_field P2.3 Owns
step_field() {
  local step="$1"
  local field="$2"
  step_line "$step" | tr '|' '\n' | sed -n "s/^ *$field: *//p" | sed 's/ *$//'
}

# Prints the title of a step: the text between the step ID and the first ":".
step_title() {
  step_line "$1" | sed -E 's/^- \[[ x]\] [^ ]+ ([^:|]*).*/\1/'
}

# Prints the paths that a step owns, one path on each line.
step_owns() {
  step_field "$1" Owns | tr ',' '\n' | sed 's/^ *//; s/ *$//' | grep -v '^$' || true
}

# Prints the steps that a step depends on, one step on each line.
step_depends() {
  step_field "$1" Depends | tr ',' '\n' | sed 's/^ *//; s/ *$//' | grep -v -E '^(none)?$' || true
}

worktree_path() {
  echo "$WORKTREE_ROOT/$1"
}

branch_name() {
  echo "refactor/step-$1"
}

# Returns success if two paths overlap.
# Two paths overlap if they are equal, or if one is a folder that contains the other.
paths_overlap() {
  local a="${1%/}"
  local b="${2%/}"
  if [ "$a" = "$b" ]; then return 0; fi
  case "$b" in "$a"/*) return 0 ;; esac
  case "$a" in "$b"/*) return 0 ;; esac
  return 1
}

# Returns success if the step owns the file.
step_owns_file() {
  local step="$1"
  local file="$2"
  local owned
  while read -r owned; do
    if paths_overlap "$owned" "$file"; then return 0; fi
  done < <(step_owns "$step")
  return 1
}
