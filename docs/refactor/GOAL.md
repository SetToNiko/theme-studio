# Refactor Goal: Theme Studio

<!-- The refactor-kit setup wrote this file. You can edit it. -->

Draft pack: junior-readable-maintainability

## Goal
After the refactor, a junior developer can read the codebase, quickly find the purpose of each module, and understand the architecture of the project. The code is as easy as possible to maintain and to extend.
Theme Studio configures colours and themes (palettes and font presets) for the system, and the programs that it is set up to configure.

## Reason
The codebase is disorganized and has much coupling. Thus, it is difficult to maintain and to add functionality. Example: `studio/src/app.rs` has 4,948 lines and changed in 46 of 96 commits.

## Success criteria
`docs/refactor/SUCCESS_CRITERIA.md` has the measurement, baseline, and target of each criterion.
1. To add a new app integration (for example a terminal other than Ghostty), you change 2 files or fewer, and no script logic.
2. One data list defines the app integrations: the files that each app reads, and the reload command of each app.
3. No code outside `studio/src/core/paths.rs` and `scripts/lib/paths.sh` contains a hard-coded system path.
4. All current functionality works: `nix flake check` passes, and the manual checklist passes.
5. Each `.rs` file and each script starts with a comment that gives its purpose. ARCHITECTURE.md lists each module with a one-line purpose.
6. After the refactor phases, one command installs Theme Studio without Nix, and `theme-switch` works after that install.
7. `cargo clippy --all-targets` with the groups pedantic, style, and correctness reports 0 warnings. Each allowed lint uses `#[expect(..., reason = "...")]`. The count never increases from one step to the next (baseline: 490).
8. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items` passes. Each function that returns `Result` has an `# Errors` section. Each function that can panic has a `# Panics` section.

## Scope
- In scope: all files: `studio/src/`, `scripts/`, `share/`, `packaging/nix/`, `flake.nix`, `docs/`.
- Out of scope:
  - Support for other Linux distributions. It is a later, separate project. No distro-specific criteria or container tests.
  - Security hardening. Theme Studio is for personal use now. Security work comes before a public release, not in this refactor.
  - New behavior other than the "install without Nix" feature phase.

## New behavior and removals
Removals (Phase R, directly after Phase 0). Each removal changes its documentation and has a handover entry in the plan (section 7):
1. The wallpaper hook: theme-switch.sh no longer runs `wallpaper-rotate refresh`.
2. The login screen: font-switch.sh no longer writes the LightDM greeter font (`/var/lib/ascii-boot`, `/etc/ascii-boot`). `packaging/nix/nixos-module.nix` and the `nixosModules` flake output are removed. Reason: users of a future public release can use different login tools.
3. The rofi picker menus: `scripts/lib/rofi-picker.sh` is removed. The switch scripts need a name (no name: a usage message, exit 2).
Kept: rofi and the lock screen as themed apps (colours and fonts).

One `[feature]` phase: **Install without Nix**.
- It is the first phase after the refactor phases. It does not start before all refactor phases are complete.
- Scope: `docs/migration.md`, section "For the refactor: what still ties it to this setup", items "Install without Nix" and "the studio's tools".
- The Nix package and the home-manager module continue to work.
- Each new behavior has documentation.

## Constraints
- All current functionality works after each step: the tests pass, and `nix build` passes.
- The Nix package and the home-manager module continue to work. (Phase R removes the NixOS module.)
- No new runtime dependencies. New development tools in the dev shell are permitted (example: shellcheck).

## Verification
All checks run with one command: `nix flake check` (add a `checks.x86_64-linux` output).
1. Rust tests: `cd studio && nix develop .. --command cargo test`, after each step. Baseline: 115 passed, 4 ignored, 1 "skipped:" line (count it with `-- --nocapture`). The numbers of ignored and skipped tests must not increase.
2. Package build: `nix build` passes after each step.
3. Golden-file tests for the switch scripts (Phase 0): temporary XDG_CONFIG_HOME, THEME_STUDIO_DATA, and THEME_STUDIO_SHARE with fixture data. Stubs for pkill, i3-msg, rofi, and fc-cache record their arguments. `diff -r` against golden files from the unchanged code. (The login-screen part of font-switch.sh is removed in Phase R.)
   Golden files change only in [removal] and [feature] steps, and only for that behavior.
4. Static checks: `shellcheck scripts/*.sh scripts/lib/*.sh` (add shellcheck to the dev shell), `cargo clippy --all-targets -- -D warnings` with pedantic enabled (ratchet, refer to criterion 7), and `cargo fmt --check`. Add clippy and rustfmt to the dev shell.
5. Extensibility check: on a temporary branch, add a sample app integration. `git diff --stat` shows 2 files or fewer, and no script logic.
6. Manual check on the real desktop after each phase that changes scripts or the preview: theme-switch and font-switch change and reload Ghostty, the i3 bar, rofi, GTK, and nvim. The studio's live preview shows the new theme.
