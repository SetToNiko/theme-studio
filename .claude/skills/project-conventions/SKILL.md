---
name: project-conventions
description: Gives the stack, commands, refactor settings, coding rules, and documentation style of this project. Load this skill before you write, refactor, review, or document code in this project.
---
<!-- The refactor-kit setup wrote this file. Each agent preloads it. Keep it shorter than approximately 60 lines. -->

## Stack and commands
- Language and framework: Rust 2024 (ratatui TUI) in studio/, bash scripts in scripts/, jq in share/, Nix packaging
- All gates: `nix flake check` (build and Rust tests, fmt, clippy, doc, shellcheck, cycles, file-limits, errors-doc, scripts, install). Decide by the exit code. One gate: `nix build .#checks.x86_64-linux.<gate>`.
- Test: `cd studio && nix develop .. --command cargo test` · scripts: `nix develop --command bash tests/scripts/run.sh` · install: `nix develop --command bash tests/install/run.sh`
- Lint: `cd studio && nix develop .. --command cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` · counts: `scripts/dev/lint-counts.sh`
- Test style: inline `#[cfg(test)] mod tests` at the bottom of each .rs file. `App` scenarios in `app/tests/`. snake_case names that state the behavior. Script behavior: a golden case in `tests/scripts/run.sh`.
- Naming: Rust standard. Scripts: kebab-case files, snake_case functions. British spelling as in the code (colour, behaviour).
- Tools not available yet: none.

## Refactor settings
- Refactor folder: `docs/refactor-fixes/`
- Goal: `docs/refactor-fixes/GOAL.md`. Principles: `docs/refactor-fixes/PRINCIPLES.md`. Plan: `docs/refactor-fixes/PLAN.md`.
- Branch: `refactor/small-fixes` · Commits: after each approved action
- Simplify step: simplifier agent · Documentation: ARCHITECTURE.md + README.md + doc comments
- Atomic steps (the rule): one `git revert` of a step's commit undoes it completely and leaves a working project (all gates pass). If one commit is not possible: a change group (`Change-Group: <name> (n/total)` trailers, ", first" and ", last" on the outer commits).
- Step size (a guideline): 5 files or fewer. Not counted: a pure move (`git mv`), new test-data files, files whose only change is an access path.
- Execution mode: parallel waves, dynamic maximum (the coordinator selects the number for each wave).
- Contributor guide: `CONTRIBUTING.md`. This refactor applies all of its settings.

## Audience: A1 Beginner (junior developer)
The reader knows Rust and bash basics. The reader possibly does not know design patterns, trait objects, advanced generics, macros, or this project.
Audience test: open a file, understand what it does and why in a few minutes, change it safely.
Each advanced technique has a plain interface in front of it. It also has a comment with its name, the reason, and how to use it, and an entry in the ARCHITECTURE.md glossary.

## Rules
Priority: correctness > audience test > readability > maintainability, modularity, error handling > extensibility.
- SRP: each module has one reason to change. Dependencies go in one direction (`ui → app → model, system, feature folders`). No cycles. No `pub use`. All `impl App` blocks stay in `app/`.
- Data-driven: app integrations, palettes, presets, and key actions are data, not branches. One source of truth.
- Max 3 jumps of indirection. Plain loops and `if` over dense iterator chains. Nesting: 4 levels or fewer.
- Files: 500 lines or fewer without the `#[cfg(test)]` module (hard). Module folders: 10 modules or fewer, not counting `mod.rs` (hard).
- Functions: guideline 30 lines. A longer function needs `// Long function: <reason>.`.
  After the reviewer checks the reason, the marker is `// Long function (reason verified, phase <N>): <reason>.`.
  A verified function is not checked again, unless an architecture rewrite of its module occurs or the function grows.
  Over 100 lines: the reason goes in `#[expect(clippy::too_many_lines, reason = "...")]`.
- Errors: each message says what failed and what to do. Errors keep their cause (anyhow `.context`).
- All system paths go through `studio/src/model/paths.rs` or `scripts/lib/paths.sh`. The two files use the same rules and give the same paths. A test compares them.
- Only `reload_apps` in `scripts/lib/integrations.sh` runs app reload commands (pkill, i3-msg, ...).
- The switch scripts do not use `set -euo pipefail`. `install.sh`, `scripts/dev/`, and `tests/` use it.
- Clippy pedantic + style + correctness, and shellcheck: zero warnings (gates). Each allowed lint uses `#[expect(..., reason = "...")]`.

## Code rules
- A refactor does not change behavior. Exceptions: a step with the tag `[bug fix]` or `[behavior]` changes only its one behavior. The commit message lists it.
- A bug fix gets a test that fails before the fix.
- Ask before you add a new dependency.
- Never run `theme-switch`, `font-switch`, `themes-switch`, or `install.sh --uninstall` on the real desktop. Use `tests/` with a temporary `HOME`, `XDG_*`, `THEME_STUDIO_DATA`, and `PREFIX`.

## Parallel mode (agents that get a worktree path)
- Work only in your worktree. Start each Bash command with `cd <worktree path> &&`.
- Change only the files that your step lists in `Owns:`.
- Never edit the plan. Never commit. Never change other branches.
- Tests run in other worktrees at the same time. Use no fixed ports, fixed temporary paths, or shared databases. Tests never call `std::env::set_var` or `remove_var`: give the function the values as arguments.
- Wave scripts: `docs/refactor-fixes/scripts/wave-start.sh`, `wave-merge.sh`, `wave-clean.sh`. Only the coordinator runs them.

## Documentation style (rustdoc, strict)
- Each .rs file starts with `//!`: a one-line purpose, then details. Each script starts with a `#` purpose comment.
- Each item has `///`: the first line is a one-sentence summary, then details.
- Sections: `# Errors` (each fn that returns Result, gate `errors-doc`), `# Panics` (can panic), `# Examples` (the use is not obvious), `# Safety` (unsafe).
- Use backticks for code and intra-doc links (`[`Type`]`) for types. Use plain language and define each term that a junior developer possibly does not know.
- Check: gate `doc` (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`).
- Write all text with the `asd-ste100` skill: documents, comments, doc comments, commit messages, and replies. Strict mode for text that an agent or a script reads (CONTRIBUTING.md, the plan, comments). Flavored mode for text for people (README.md, ARCHITECTURE.md, the user guide, commit messages). Run its linter (`scripts/ste-lint.py`) on each changed document.
