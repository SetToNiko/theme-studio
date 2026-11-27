# Handoff: Theme Studio

Use this file to continue the work on Theme Studio on another machine, or in a new session. It gives the state, the next steps, and the setup that the work needs. Written on 2026-11-27.

## 1. State

- The maintainability refactor is complete. `main` holds it (merge `068da69`). `docs/refactor/BEFORE_AFTER.md` shows the modules before and after, with the key numbers.
- After the merge, `main` got `CONTRIBUTING.md` (`a3217ea`): the gates, the rules, and a map of the code.
- The user tested the refactored studio on the laptop. It works as before.
- The branch `refactor/maintainability` is merged. You can delete it.

Where the details are:

| Topic | File |
|---|---|
| The checks that must pass, the rules, the module map, parallel work | `CONTRIBUTING.md` |
| How the code works, a walk-through, the glossary | `ARCHITECTURE.md` |
| The goal and the principles of the refactor | `docs/refactor/GOAL.md`, `docs/refactor/PRINCIPLES.md` |
| The findings, the history, the outcome | `docs/refactor/PLAN.md` (sections 4, 8, 10) |
| What changed | `docs/refactor/BEFORE_AFTER.md` |

## 2. Next steps

Do them in this order.

### Step 1: Small fixes

Each fix is small, and most of them touch different folders. Recommended: one short refactor round with the refactor kit in parallel mode. This round is also the first real run of the kit's wave scripts.

| Fix | What to do | Source |
|---|---|---|
| `settings_page` uses `app` | Move `BhRow` and `LookId` from `app/state/nav.rs` into `settings_page`. Move `draw_browse` into `ui/page_bodies/settings/`, and give it the hit types as arguments. Then no feature folder uses `app`. | `CONTRIBUTING.md`, section 7 |
| `themes-switch` half-switches | A theme with a missing palette applies the fonts, then fails, and nothing reloads. Check the palette before the script changes anything. | finding 17 |
| The install folder is also the previews folder | With the default `PREFIX`, `~/.local/share/theme-studio` holds the installed files and the downloaded preview fonts. Keep the previews in a separate folder. | finding 29 |
| A test that never runs | `real_configs_parse` in `system/desktop.rs` looks in the data folder, but the desktop configs are in `store.desktop`. | finding 16 |
| XDG rules | `paths.rs` reads the XDG base folders in three different ways, and has no tests for them. No test compares `scripts/lib/paths.sh` with `paths.rs`. | findings 18, 19, 20 |
| Settings page details | `draft_changes` returns at least 1 when only other keys differ (`n.max(1)`). Confirm on a Gap row moves the cursor to row 0. | findings 25, 26 |
| Old documents | `docs/plan.md` and `docs/migration.md` describe the setup before the refactor. Archive them, or update them. | `PLAN.md`, section 8 |
| A missing test | `main.rs` `--init-config` has no test. | `CONTRIBUTING.md`, section 7 |

### Step 2: Terminal independence

A new refactor with the kit. Theme Studio works with terminals other than Ghostty. The terminal becomes one entry in `share/integrations.json`, like the other programs.
- `CONTRIBUTING.md`, section 7, lists each Ghostty-specific place.
- Include the deferred note N1: the `Preview` type lives in two folders (`preview_term/` and `ui/desktop_preview.rs`), so 7 fields are `pub(crate)`. Put the type in one folder.

### Step 3: Preview per app

The next feature. The desktop preview reads `share/integrations.json`, so each configured program also shows in the preview. Do it after step 2.

### Step 4: Before a public release

Support for other Linux distributions, and a security review. Both are separate projects.

## 3. Decisions to keep

The user made these decisions. A new session must follow them.

- **One refactor kit.** `/refactor-kit` does sequential and parallel work. The first question of its setup selects the mode, and the second selects the number of agents (a number, or dynamic). Do not make a separate parallel kit.
- **All text with the asd-ste100 skill.** The kit writes each text with this skill: the files, the comments, the commit messages, the instructions to agents, and the messages to the user. Strict mode for text that an agent or a script reads. Flavored mode for text for people. Run the linter of the skill on each written file.
- **Commits.** Each commit is atomic. Commit only your own changes. Other uncommitted work in a repository stays as it is.
- **Real system.** Never run `theme-switch`, `font-switch`, `themes-switch`, or `install.sh --uninstall` on the real desktop without a temporary `HOME` and `PREFIX`. Use the test harnesses in `tests/`.
- **Login screen.** Theme Studio does not manage the login screen. The dotfiles set the greeter font (`modules/ascii-boot.nix`).

## 4. Set up another machine

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
cd ~/src/theme-studio && nix flake check
```
`nix flake check` runs all gates. It must pass before you change anything.

### The refactor kit and the skills

The skills are in the private repository `SetToNiko/skills`. `asd-ste100` is a submodule.

1. Get the skills:
   ```
   git clone --recurse-submodules git@github.com:SetToNiko/skills.git ~/agent-skills
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

## 5. Start the next step

1. Open a session in `~/src/theme-studio`.
2. Tell Claude: "Read `docs/refactor/HANDOFF.md`."
3. Type `/refactor-kit`. The kit reports that the last refactor is complete, and asks if you want a new refactor goal. Answer yes. Then the setup starts:
   - The first question selects the mode. For step 1, select parallel.
   - The setup finds `CONTRIBUTING.md` and asks which of its settings this refactor applies.
   - For the refactor folder, select a new folder, for example `docs/refactor-fixes/`. Then the files of the first refactor stay as history in `docs/refactor/`.
4. Use section 2 of this file for the goal and the scope.
