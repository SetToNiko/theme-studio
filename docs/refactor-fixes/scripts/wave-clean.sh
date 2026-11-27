#!/usr/bin/env bash
# Removes the worktrees and branches of a wave.
#
# Usage: docs/refactor-fixes/scripts/wave-clean.sh P2.1 P2.2
#        docs/refactor-fixes/scripts/wave-clean.sh --force P2.3
#
# Without --force, the script removes only the steps that are merged.
# With --force, it also removes the steps that are not merged. Their changes are lost.

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

force="no"
if [ "${1:-}" = "--force" ]; then
  force="yes"
  shift
fi
if [ $# -lt 1 ]; then die "Give one or more step IDs. Example: wave-clean.sh P2.1 P2.2"; fi

cd "$PROJECT_ROOT" || exit 1

for step in "$@"; do
  path="$(worktree_path "$step")"
  branch="$(branch_name "$step")"

  if ! git show-ref --quiet "refs/heads/$branch"; then
    if [ -d "$path" ]; then git worktree remove --force "$path"; fi
    echo "$step: removed (no branch)"
    continue
  fi

  # A worktree with uncommitted changes has work that is not merged.
  has_uncommitted="no"
  if [ -d "$path" ] && [ -n "$(git -C "$path" status --porcelain)" ]; then has_uncommitted="yes"; fi

  if [ "$has_uncommitted" = "no" ] && git merge-base --is-ancestor "$branch" "$INTEGRATION_BRANCH"; then
    if [ -d "$path" ]; then git worktree remove "$path"; fi
    git branch --quiet -d "$branch"
    echo "$step: removed (merged)"
  elif [ "$force" = "yes" ]; then
    if [ -d "$path" ]; then git worktree remove --force "$path"; fi
    git branch --quiet -D "$branch"
    echo "$step: removed (not merged, --force)"
  else
    echo "$step: kept, because it is not merged. Use --force to remove it."
  fi
done
