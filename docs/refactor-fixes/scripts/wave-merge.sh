#!/usr/bin/env bash
# Merges the steps of a wave into the integration branch, one step at a time.
#
# Usage: docs/refactor-fixes/scripts/wave-merge.sh P2.1 P2.2 P2.3
#
# For each step, in the order that you give:
#   a. Runs the tests in the worktree of the step. Then commits the changes of the step.
#   b. Checks that the step changed only the files that it owns.
#   c. Merges the branch of the step into the integration branch.
#   d. Runs the tests on the integration branch. If they fail, undoes the merge.
#   e. Marks the step as done in the plan and commits the plan.
# If a check fails, the script skips the step and continues with the next step.
# The skipped steps stay in their worktrees. You can examine them, then run wave-clean.sh.

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

if [ $# -lt 1 ]; then die "Give one or more step IDs. Example: wave-merge.sh P2.1 P2.2"; fi

cd "$PROJECT_ROOT" || exit 1

if [ "$(git branch --show-current)" != "$INTEGRATION_BRANCH" ]; then
  die "Switch to the branch $INTEGRATION_BRANCH first."
fi
if [ -n "$(git status --porcelain)" ]; then
  die "The project has uncommitted changes. Commit them first."
fi

mkdir -p "$LOG_DIR"
merged=()
skipped=()

# Records a skipped step and its reason.
skip_step() {
  skipped+=("$1: $2")
  echo "  SKIPPED $1: $2"
}

for step in "$@"; do
  echo "Step $step"
  path="$(worktree_path "$step")"
  branch="$(branch_name "$step")"
  title="$(step_title "$step")"
  log="$LOG_DIR/$step.log"

  if [ ! -d "$path" ]; then skip_step "$step" "no worktree at $path"; continue; fi

  # a. Test and commit in the worktree.
  if ! (cd "$path" && bash -c "$TEST_CMD") > "$log" 2>&1; then
    skip_step "$step" "tests fail in the worktree. Log: $log"
    continue
  fi
  if [ -n "$(git -C "$path" status --porcelain)" ]; then
    git -C "$path" add -A
    git -C "$path" commit --quiet -m "Refactor: $step $title"
  fi

  base="$(git merge-base "$INTEGRATION_BRANCH" "$branch")"
  changed_files="$(git diff --name-only "$base" "$branch")"
  if [ -z "$changed_files" ]; then skip_step "$step" "the step changed no files"; continue; fi

  # b. Check the ownership of each changed file.
  not_owned=""
  while read -r file; do
    if ! step_owns_file "$step" "$file"; then not_owned="$not_owned $file"; fi
  done <<< "$changed_files"
  if [ -n "$not_owned" ]; then
    skip_step "$step" "it changed files that it does not own:$not_owned"
    continue
  fi

  # c. Merge.
  if ! git merge --quiet --no-ff -m "Merge $step: $title" "$branch" >> "$log" 2>&1; then
    git merge --abort
    skip_step "$step" "merge conflict. Run the step again on the new integration branch."
    continue
  fi

  # d. Test on the integration branch.
  if ! bash -c "$TEST_CMD" >> "$log" 2>&1; then
    # ORIG_HEAD is the commit before the merge. This removes only the merge of this step.
    git reset --quiet --hard ORIG_HEAD
    skip_step "$step" "tests fail after the merge. Log: $log"
    continue
  fi

  # e. Mark the step as done.
  sed -i "s/^- \[ \] $(step_regex "$step") /- [x] $step /" "$PLAN_FILE"
  git commit --quiet -am "Plan: mark $step as done"
  merged+=("$step")
  echo "  MERGED"
done

echo
echo "Merged: ${merged[*]:-none}"
if [ ${#skipped[@]} -gt 0 ]; then
  echo "Skipped:"
  for line in "${skipped[@]}"; do echo "  $line"; done
  exit 1
fi
