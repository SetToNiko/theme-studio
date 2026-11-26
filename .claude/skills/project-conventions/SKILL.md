---
name: project-conventions
description: Gives the stack, commands, refactor settings, coding rules, and documentation style of this project. Load this skill before you write, refactor, review, or document code in this project.
---
<!-- The refactor-kit setup wrote this file. Each agent preloads it. Keep it shorter than approximately 60 lines. -->

## Stack and commands
- Language and framework: Rust 2024 (ratatui TUI) in studio/, bash scripts in scripts/, jq in share/, Nix packaging
- Build: `nix build` (dev: `cd studio && nix develop .. --command cargo build`)
- Test: `cd studio && nix develop .. --command cargo test` (later: `nix flake check`)
- Lint: `cd studio && nix develop .. --command cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` · `shellcheck scripts/*.sh scripts/lib/*.sh`
- Test style: inline `#[cfg(test)] mod tests` at the bottom of each .rs file. snake_case names that state the behavior.
- Naming: Rust standard. Scripts: kebab-case files, snake_case functions. British spelling as in the code (colour, behaviour).
- Tools not available yet: none. (jscpd is not in nixpkgs: duplicates use `lizard -Eduplicate`.)

## Refactor settings
- Refactor folder: `docs/refactor/` (GOAL.md, PRINCIPLES.md, SUCCESS_CRITERIA.md, PLAN.md)
- Branch: `refactor/maintainability` · Commits: after each approved action
- Simplify step: simplifier agent · Documentation: ARCHITECTURE.md + README.md + doc comments
- Atomic steps (the rule): one `git revert` of a step's commit undoes it completely and leaves a working project (tests pass). If one commit is not possible: a change group (`Change-Group: <name> (n/total)` trailers, ", first" and ", last" on the outer commits).
- Step size (a guideline): 5 files or fewer. Not counted: a pure move (`git mv`), new test-data files, files whose only change is an access path.

## Audience: A1 Beginner (junior developer)
The reader knows Rust and bash basics. The reader possibly does not know design patterns, trait objects, advanced generics, macros, or this project.
Audience test: open a file, understand what it does and why in a few minutes, change it safely.
Each advanced technique: hidden behind a plain interface, a comment with its name, the reason, and how to use it, and an entry in the ARCHITECTURE.md glossary.

## Rules
Priority: correctness > audience test > readability > maintainability, modularity, error handling > extensibility.
- SRP: each module has one reason to change. Dependencies go in one direction. No cycles.
- Data-driven: app integrations, palettes, and presets are data, not branches. One source of truth.
- Extensibility only where variation is real (app integrations).
- Max 3 jumps of indirection. Plain loops and `if` over dense iterator chains. Nesting: 4 levels or fewer.
- Files: 500 lines or fewer without the `#[cfg(test)]` module.
- Functions: guideline 30 lines. A longer function needs `// Long function: <reason>.`
  After the reviewer checks the reason, the marker is `// Long function (reason verified, phase <N>): <reason>.`
  A verified function is not checked again, unless an architecture rewrite of its module occurs or the function grows.
  Over 100 lines: the reason goes in `#[expect(clippy::too_many_lines, reason = "...")]`.
- Errors: each message says what failed and what to do. Errors keep their cause (anyhow `.context`).
- All system paths go through `studio/src/paths.rs` or `scripts/lib/paths.sh`.
- Only one location runs app reload commands (pkill, i3-msg, ...).
- Clippy pedantic + style + correctness: the functions that a step writes or changes have no warnings. The total never increases. Phase 6 makes whole files clean. Each allowed lint uses `#[expect(..., reason = "...")]`.
- Changed scripts pass shellcheck.

## Code rules
- A refactor does not change behavior. Exception: a step with the tag `[feature]`.
- Ask before you add a new dependency.

## Documentation style (rustdoc, strict)
- Each .rs file starts with `//!`: a one-line purpose, then details. Each script starts with a `#` purpose comment.
- Each item has `///`: the first line is a one-sentence summary, then details.
- Sections: `# Errors` (each fn that returns Result), `# Panics` (can panic), `# Examples` (the use is not obvious), `# Safety` (unsafe).
- Use backticks for code and intra-doc links (`[`Type`]`) for types. Use plain language and define each term that a junior developer possibly does not know.
- Check: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`.
