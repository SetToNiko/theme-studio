# Refactor Goal: Theme Studio, small fixes

Draft pack: small-fixes-round

<!-- The refactor-kit setup wrote this file. You can edit it. Keep it shorter than approximately 40 lines. -->

## Goal
After this refactor, the open items of `CONTRIBUTING.md` section 7 that are not about the terminal have a fix. Each feature folder obeys the dependency direction. The scripts and the studio use the same path rules, and tests check them.

## Reason
The first refactor (`docs/refactor/`) left 8 small open items. Some items are bugs: `themes-switch` can leave the desktop half-switched, and an uninstall can remove the preview fonts. Some tests do not run or do not exist. The terminal-independence refactor (handoff step 2) starts after these fixes, from clean code.

## Success criteria
1. No production code in `studio/src/settings_page/` uses `crate::app`. `BhRow` and `LookId` are in `settings_page`. `draw_browse` is in `ui/page_bodies/settings/` and gets the hit types as arguments.
2. `themes-switch` with a missing palette exits 1 and changes no file. The golden case `themes-switch-missing-palette` shows this.
3. The preview fonts are in `$XDG_CACHE_HOME/theme-studio/previews`. At start, the studio moves an old `$XDG_DATA_HOME/theme-studio/previews` folder one time. The install test shows that no install and no uninstall touches the previews folder.
4. The test `real_configs_parse` reads `store.desktop`. It runs when the desktop configs exist.
5. All XDG base folders in `paths.rs` use one rule: use the variable only if it is an absolute path, else use the default under `HOME`. `scripts/lib/paths.sh` uses the same rule. Each path function has a test. A test compares `paths.sh` with `paths.rs` for each path that both files give.
6. `draft_changes` returns 0 when only keys outside "settings" and "keymaps" differ. The `Confirm` action on a Gap row does not move the cursor.
7. `docs/plan.md` and `docs/migration.md` are in `docs/archive/`, with a note at the top. Each open item from them is in `CONTRIBUTING.md` section 7.
8. `main.rs` `--init-config` has a test.
9. `CONTRIBUTING.md` section 7 does not list the fixed items. `nix flake check` passes.

## Scope
- In scope: the 8 fixes of `docs/refactor/HANDOFF.md`, section 2, step 1. Findings 16, 17, 18, 19, 20, 25, 26, 29 of `docs/refactor/PLAN.md`. The documents that these fixes change.
- Out of scope:
  - Terminal independence and note N1 (the `Preview` type in two folders): handoff step 2.
  - The preview of each app: handoff step 3.
  - Support for other Linux distributions, and a security review: handoff step 4.

## New behavior and removals
No new features. No removals. Each bug fix is a `[bug fix]` step with a test that fails before the fix. The new previews folder (criterion 3) and the XDG rule (criterion 5) are `[behavior]` steps. Each commit message lists the changed behavior.

## Constraints
- The project works after each step: `nix flake check` passes.
- No new runtime dependencies. You can add development tools to the dev shell.
- Never run `theme-switch`, `font-switch`, `themes-switch`, or `install.sh --uninstall` on the real desktop. Use the harnesses in `tests/` with a temporary `HOME` and `PREFIX`.

## Verification
1. `nix flake check` after each step and after each merge. Decide by the exit code.
2. Rust tests: 195 tests, 4 ignored. The number of ignored tests and of "skipped:" lines must not increase.
3. Script golden cases (`tests/scripts/run.sh`) change only in `[bug fix]` and `[behavior]` steps, and only for that change.
4. Manual check on the laptop after the last step: open the studio, show a font preview, and switch a theme.

## Contributor guide
`CONTRIBUTING.md`. This refactor applies all of its settings.
