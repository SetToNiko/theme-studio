#!/usr/bin/env bash
# Starts a wave. Makes one git worktree and one branch for each step.
# Each agent then works in its own worktree, so the agents do not change the same files.
#
# Usage: docs/refactor-fixes/scripts/wave-start.sh P2.1 P2.2 P2.3
#
# The script stops before it changes anything if one of these checks fails:
#   - You are on the integration branch, and it has no uncommitted changes.
#   - Each step is in the plan, is not done, and has an Owns field.
#   - Each step that a step depends on is done.
#   - No two steps own the same file.
#   - No step owns the plan file.

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

if [ $# -lt 1 ]; then die "Give one or more step IDs. Example: wave-start.sh P2.1 P2.2"; fi
# MAX_PARALLEL=0 means a dynamic maximum: the coordinator selects the number for each wave.
if [ "$MAX_PARALLEL" -gt 0 ] && [ $# -gt "$MAX_PARALLEL" ]; then die "A wave has $MAX_PARALLEL steps or fewer. You gave $#."; fi

cd "$PROJECT_ROOT" || exit 1

if [ "$(git branch --show-current)" != "$INTEGRATION_BRANCH" ]; then
  die "Switch to the branch $INTEGRATION_BRANCH first."
fi
if [ -n "$(git status --porcelain)" ]; then
  die "The project has uncommitted changes. Commit them first."
fi

# 1. Check each step.
for step in "$@"; do
  if [ -z "$(step_line "$step")" ]; then die "Step $step is not in $PLAN_FILE."; fi
  if step_is_done "$step"; then die "Step $step is already done."; fi
  if [ -z "$(step_owns "$step")" ]; then die "Step $step has no Owns field."; fi
  if step_owns_file "$step" "$PLAN_FILE"; then die "Step $step owns $PLAN_FILE. Only the coordinator changes the plan."; fi

  while read -r dependency; do
    if ! step_is_done "$dependency"; then
      die "Step $step depends on $dependency. Merge $dependency first."
    fi
  done < <(step_depends "$step")

  if [ -e "$(worktree_path "$step")" ]; then die "The worktree of $step exists. Run wave-clean.sh $step first."; fi
  if git show-ref --quiet "refs/heads/$(branch_name "$step")"; then die "The branch of $step exists. Run wave-clean.sh $step first."; fi
done

# 2. Check that no two steps own the same file.
steps=("$@")
for ((i = 0; i < ${#steps[@]}; i++)); do
  for ((j = i + 1; j < ${#steps[@]}; j++)); do
    first="${steps[i]}"
    second="${steps[j]}"
    while read -r path_a; do
      while read -r path_b; do
        if paths_overlap "$path_a" "$path_b"; then
          die "Steps $first and $second both own '$path_a' / '$path_b'. Put them in different waves."
        fi
      done < <(step_owns "$second")
    done < <(step_owns "$first")
  done
done

# 3. Make all worktrees first. Then print the list.
# Reason: if the caller stops reading the output early, all worktrees still exist.
mkdir -p "$WORKTREE_ROOT" "$LOG_DIR"
for step in "$@"; do
  git worktree add --quiet -b "$(branch_name "$step")" "$(worktree_path "$step")" HEAD
done

echo "Wave started. Give each agent its step ID and its worktree path:"
for step in "$@"; do
  echo "  $step  $(worktree_path "$step")"
done
