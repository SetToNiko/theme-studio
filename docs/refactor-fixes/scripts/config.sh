# shellcheck shell=bash
# shellcheck disable=SC2034 # The other wave scripts read these variables.
# Settings for the wave scripts. The refactor-kit setup wrote this file.
# The other scripts load this file. You do not run it.

# The command that runs all tests. It must exit with 0 when all tests pass.
TEST_CMD="nix flake check"

# The plan file, relative to the project root.
PLAN_FILE="docs/refactor-fixes/PLAN.md"

# The branch that collects the merged steps.
INTEGRATION_BRANCH="refactor/kit-rules"

# The folder for the worktrees, relative to the project root.
# It is outside the project, so the worktrees do not appear in "git status".
WORKTREE_ROOT="../$(basename "$PROJECT_ROOT")-worktrees"

# The maximum number of steps in one wave. 0: dynamic (the coordinator selects the number for each wave).
MAX_PARALLEL=0
