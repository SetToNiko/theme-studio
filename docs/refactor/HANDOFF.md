# Handoff: Theme Studio

Use this file to continue the work on Theme Studio on another machine, or in a new session. It gives the state, the next steps, and the setup that the work needs. Updated on 2026-11-27.

## 1. State

- The maintainability refactor is complete. `main` holds it (merge `068da69`). `docs/refactor/BEFORE_AFTER.md` shows the modules before and after, with the key numbers.
- After the merge, `main` got `CONTRIBUTING.md` (`a3217ea`): the gates, the rules, and a map of the code.
- The user tested the refactored studio on the laptop. It works as before.
- `main` has the branch `refactor/maintainability` in it. You can delete the branch.
- **A second refactor round has a setup, but no plan.** The branch `refactor/small-fixes` holds it. Its folder is `docs/refactor-fixes/`: `GOAL.md`, `PRINCIPLES.md`, `SUCCESS_CRITERIA.md` (with a measured baseline), and the wave scripts. The mode is parallel, with a dynamic number of agents.
- **After that setup, the user changed the refactor kit.** The setup on the branch is from the old kit, so it does not have the new rules. Section 2 tells what to do.

Where the details are:

| Topic | File |
|---|---|
| The checks that must pass, the rules, the module map, parallel work | `CONTRIBUTING.md` |
| How the code works, a walk-through, the glossary | `ARCHITECTURE.md` |
| The goal and the principles of the first refactor | `docs/refactor/GOAL.md`, `docs/refactor/PRINCIPLES.md` |
| The findings, the history, the outcome | `docs/refactor/PLAN.md` (sections 4, 8, 10) |
| What changed | `docs/refactor/BEFORE_AFTER.md` |
| The setup of the second round | `docs/refactor-fixes/` on the branch `refactor/small-fixes` |

## 2. The new rules of the refactor kit

The kit (`SetToNiko/skills`, commits `bca1c95`, `06de00d`, `8271127`) has these new rules. Theme Studio does not obey them yet.

**For all languages:**
- **Lines of code.** A line limit counts only lines of code. Empty lines and lines with only a comment (doc comments too) do not count. A line with code and a comment counts as one line. Measure with lizard (NLOC) or tokei (Code). The gate `scripts/dev/file-limits.sh` counts all production lines now. It must change.
- **Group the code by purpose.** Put the lines that do one task together, with one empty line between two groups. A limit is never a reason to delete an empty line or a comment.

**For Rust:**
- **`mod.rs`** holds only module declarations: public (`pub mod`), private (`mod`), or internal (`pub(crate)`, `pub(super)`, `pub(in <path>)`). It also holds a `//!` header and a `///` comment on each declaration. No `use`, no code, no `pub use`, no tests. Expose only what the consumer needs: use the smallest visibility that works. No exceptions. A gate checks it.
- **Unit tests** are in the file of the function that they test, in a `#[cfg(test)] mod tests` at the end. Each test has `#[test]`.
- **Integration tests** are in `tests/` of the crate root: `studio/tests/`. They use only the public API, so the crate needs a library target (`studio/src/lib.rs`).
- **The Rust API Guidelines** (https://rust-lang.github.io/api-guidelines/) are a MUST for each agent and each action. The kit has the checklist with the item IDs (`reference/rust-api-guidelines.md`). The setup copies it to `.claude/skills/project-conventions/rust-api-guidelines.md`. `# Examples` is now on each public item (C-EXAMPLE).
- **Checks for each Rust change:** `cargo nextest run` and `cargo test --doc`, `cargo llvm-cov nextest` (each changed function has a test), `cargo mutants --in-diff` (catch each missed mutant), and `cargo +nightly fuzz run` for each changed function that parses outside input. The output goes to a temporary folder.

**Theme Studio today** (measured on 2026-11-27):

| Rule | Now |
|---|---|
| `mod.rs` holds only declarations | 19 of 22 `mod.rs` files have other lines. The largest: `keymap/mod.rs` 652, `fonts/mod.rs` 583, `config/mod.rs` 418, `settings_page/mod.rs` 365, `app/mod.rs` 342. |
| Unit tests in the file of the function | 8 test files are separate (`tests.rs`, `app/tests/`). |
| Integration tests in `studio/tests/` | No `studio/tests/`. No `src/lib.rs`. |
| Tools | cargo-llvm-cov: yes. cargo-nextest, cargo-mutants, cargo-fuzz, a nightly toolchain: no. |
| API guidelines | Not checked yet. |

## 3. Next steps

Do them in this order.

### Step 1: Apply the new rules, and the small fixes

The user wants Theme Studio to obey the new rules. Do it in a fresh session, with the refactor kit.

1. Start the session in `~/src/theme-studio`, on the branch `refactor/small-fixes`.
2. Type `/refactor-kit setup`. The setup finds the files of `docs/refactor-fixes/` and asks for each part: keep it, or do it again.
   - Do the goal (part 3), the principles (part 4), the conventions (part 5), and the success criteria (part 6) again. The kit then writes the new rules and the agent files of the new templates, and the API checklist.
   - Decide with the user: one refactor for the new rules and the small fixes, or two refactors (the rules first). The rules change most module folders, so in parallel mode they are a restructure, not small fixes.
   - Phase 0 adds cargo-nextest, cargo-mutants, cargo-fuzz, and a nightly toolchain (only for cargo-fuzz) to the dev shell. It also adds the `mod.rs` gate, and changes `file-limits.sh` to count lines of code.
3. The small fixes stay in the goal (`docs/refactor-fixes/GOAL.md` has them as criteria 1 to 9):

| Fix | What to do | Source |
|---|---|---|
| `settings_page` uses `app` | Move `BhRow` and `LookId` from `app/state/nav.rs` into `settings_page`. Move `draw_browse` into `ui/page_bodies/settings/`, and give it the hit types as arguments. Then no feature folder uses `app`. | `CONTRIBUTING.md`, section 7 |
| `themes-switch` half-switches | A theme with a missing palette applies the fonts, then fails, and nothing reloads. Check the palette before the script changes anything. | finding 17 |
| The install folder is also the previews folder | Move the previews to `$XDG_CACHE_HOME/theme-studio/previews`. At start, the studio moves the old folder one time. (`preview_cache_dir` uses `XDG_DATA_HOME` now.) | finding 29 |
| A test that never runs | `real_configs_parse` in `system/desktop.rs` looks in the data folder, but the desktop configs are in `store.desktop`. | finding 16 |
| XDG rules | `paths.rs` reads the XDG base folders in three ways. Use one rule: a variable counts only if it is an absolute path. `paths.sh` uses the same rule. Each path function gets a test, and a test compares `paths.sh` with `paths.rs`. Tests never call `set_var`. | findings 18, 19, 20 |
| Settings page details | `draft_changes` returns at least 1 when only other keys differ (`n.max(1)`). The `Confirm` action on a Gap row moves the cursor to row 0. | findings 25, 26 |
| Old documents | Move `docs/plan.md` and `docs/migration.md` to `docs/archive/`, with a note at the top. | `PLAN.md`, section 8 |
| A missing test | `main.rs` `--init-config` has no test. | `CONTRIBUTING.md`, section 7 |

### Step 2: Terminal independence

A new refactor with the kit. Theme Studio works with terminals other than Ghostty. The terminal becomes one entry in `share/integrations.json`, like the other programs.
- `CONTRIBUTING.md`, section 7, lists each Ghostty-specific place.
- Include the deferred note N1: the `Preview` type lives in two folders (`preview_term/` and `ui/desktop_preview.rs`), so 7 fields are `pub(crate)`. Put the type in one folder.

### Step 3: Preview per app

The next feature. The desktop preview reads `share/integrations.json`, so each configured program also shows in the preview. Do it after step 2.

### Step 4: Before a public release

Support for other Linux distributions, and a security review. Both are separate projects.

## 4. Decisions to keep

The user made these decisions. A new session must follow them.

- **One refactor kit.** `/refactor-kit` does sequential and parallel work. The first question of its setup selects the mode, and the second selects the number of agents (a number, or dynamic). Do not make a separate parallel kit.
- **All text with the asd-ste100 skill.** The kit writes each text with this skill: the files, the comments, the commit messages, the instructions to agents, and the messages to the user. Strict mode for text that an agent or a script reads. Flavored mode for text for people. Run the linter of the skill on each written file.
- **Commits.** Each commit is atomic. Commit only your own changes. Other uncommitted work in a repository stays as it is.
- **Real system.** Never run `theme-switch`, `font-switch`, `themes-switch`, or `install.sh --uninstall` on the real desktop without a temporary `HOME` and `PREFIX`. Use the test harnesses in `tests/`.
- **Login screen.** Theme Studio does not manage the login screen. The dotfiles set the greeter font (`modules/ascii-boot.nix`).
- **Lines of code, and the Rust rules** of section 2. They are rules of the kit for each project.
- **Decisions of the second-round setup:** the previews go to the cache folder, and the studio moves the old folder one time. The old documents go to `docs/archive/`. The XDG rule is the rule of the XDG specification (absolute paths only). All settings of `CONTRIBUTING.md` apply.

## 5. Set up another machine

### Theme Studio itself (NixOS with the dotfiles)

The dotfiles get Theme Studio from GitHub (`github:SetToNiko/theme-studio`, branch `main`).

1. Get the dotfiles and the configs. First check for local changes:
   ```
   cd ~/dotfiles-nixos && git status && git pull
   cd ~/dotfiles-nixos/configs && git status && git pull
   ```
2. Build and switch. Use the host of the machine (`desktop` or `laptop`):
   ```
   sudo nixos-rebuild switch --flake ~/dotfiles-nixos#desktop
   ```
3. Reload i3 (`$mod+Shift+r`). Then `$mod+p` opens the palette menu.
4. Check the login font: `modules/ascii-boot.nix` sets "JetBrainsMono Nerd Font Mono", 14pt. The font file must be in `/var/lib/ascii-boot/font/`. If the login screen shows another font, copy the file there.

### The checkout for development

```
git clone git@github.com:SetToNiko/theme-studio.git ~/src/theme-studio
cd ~/src/theme-studio && git switch refactor/small-fixes && nix flake check
```
`nix flake check` runs all gates. It must pass before you change anything.

### The refactor kit and the skills

The skills are in the private repository `SetToNiko/skills`. `asd-ste100` is a submodule.

1. Get the skills. On a machine that has them, pull the new kit:
   ```
   git clone --recurse-submodules git@github.com:SetToNiko/skills.git ~/agent-skills
   cd ~/agent-skills && git pull --recurse-submodules
   ```
2. Install your own skills as links, so that a change in `~/agent-skills` is live at once:
   ```
   mkdir -p ~/.claude/skills
   for n in asd-ste100 refactor-kit ux-writing localization-review domain-modeling find-skills grilling grill-with-docs; do
     ln -sfn ~/agent-skills/$n ~/.claude/skills/$n
   done
   ```
   `mentor` comes from the claude.ai account. It syncs by itself.
3. Load asd-ste100 in each session: add this hook to `~/.claude/settings.json`. Merge it with the settings that exist:
   ```json
   {
     "hooks": {
       "SessionStart": [
         {
           "matcher": "startup|resume|clear|compact",
           "hooks": [
             {
               "type": "command",
               "command": "printf \"The asd-ste100 skill (Simplified Technical English) is loaded at session start. Its base directory is %s/agent-skills/asd-ste100; the linter is scripts/ste-lint.py there.\\n\\n\" \"$HOME\"; cat \"$HOME/agent-skills/asd-ste100/SKILL.md\"",
               "timeout": 10,
               "statusMessage": "Loading asd-ste100"
             }
           ]
         }
       ]
     }
   }
   ```
4. Start a new session. Type `/refactor-kit status` to check that Claude finds the kit.

The linter of asd-ste100 needs Python. If `python3` is not on `PATH`, run it with `nix run nixpkgs#python3 -- -I ~/agent-skills/asd-ste100/scripts/ste-lint.py <file>`.

The goal packs of the kit are in `~/.claude/refactor-kit/library/` (not in a repository). The second-round setup saved the pack `small-fixes-round` there. On another machine, the pack is not available. The setup then starts without it.

## 6. Start the next step

1. Open a session in `~/src/theme-studio`, on the branch `refactor/small-fixes`.
2. Tell Claude: "Read `docs/refactor/HANDOFF.md`."
3. Type `/refactor-kit setup`. Follow section 3, step 1.
