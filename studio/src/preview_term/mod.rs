//! The live terminal behind the preview: the hidden programs and their draft palette.
//!
//! [`Preview`] starts nvim (your config), an interactive shell (starship) and yazi
//! in hidden terminals ([`pane::Pane`]) and writes the draft palette to a colours
//! file that they read. `ui/desktop_preview.rs` draws their screens with the draft
//! palette, so nothing has to be reloaded except nvim, which re-reads the draft
//! colours file itself. The frame and click targets of the Preview pane are in
//! `ui/panes/preview.rs`.

pub mod pane;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::CommandBuilder;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::desktop::{self, BarBlock, Roles};
use crate::model::palette::{KEYS, Palette};
use pane::{Pane, key_bytes};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaneId {
    Nvim,
    Shell,
    Yazi,
}

/// What the preview pane shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Desktop,
    Swatches,
}

pub struct Preview {
    pub(crate) configs: PathBuf,
    /// Where the shell and yazi start: a git checkout (for `git log`), else home.
    workdir: PathBuf,
    draft_dir: PathBuf,
    draft: PathBuf,
    written: Option<Palette>,
    dirty: Arc<AtomicBool>,
    pub(crate) nvim: Option<Pane>,
    pub(crate) shell: Option<Pane>,
    pub(crate) yazi: Option<Pane>,
    pub(crate) errors: Vec<String>,
    pub(crate) roles: Roles,
    pub(crate) bar: Arc<Mutex<Option<Vec<BarBlock>>>>,
    pub mode: Mode,
    /// The right-hand window shows yazi instead of the shell.
    pub show_yazi: bool,
    pub show_rofi: bool,
    /// Pane receiving the keyboard, if any (F12 cycles).
    pub focus: Option<PaneId>,
    /// Where each visible pane was drawn, for mouse clicks.
    pub rects: Vec<(PaneId, Rect)>,
    /// Programs aren't started (tests).
    pub(crate) headless: bool,
}

impl Preview {
    pub fn new(configs: &Path) -> Preview {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        remove_stale_drafts(&base);
        // pid, plus a counter so several previews in one process (tests) differ
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let draft_dir = base.join(format!("theme-studio-{}-{n}", std::process::id()));
        let _ = std::fs::create_dir_all(&draft_dir);
        let dirty = Arc::new(AtomicBool::new(true));
        let bar = Arc::new(Mutex::new(None));
        {
            let (bar, dirty, configs) = (bar.clone(), dirty.clone(), configs.to_path_buf());
            std::thread::spawn(move || {
                let blocks = desktop::read_bar(&configs, Duration::from_secs(3));
                *bar.lock().unwrap() = Some(blocks);
                dirty.store(true, Ordering::Relaxed);
            });
        }
        Preview {
            configs: configs.to_path_buf(),
            // the demo shell's `git log`: the checkout, else home
            workdir: crate::model::paths::demo_dir(),
            draft: draft_dir.join("colors.sh"),
            draft_dir,
            written: None,
            dirty,
            nvim: None,
            shell: None,
            yazi: None,
            errors: Vec::new(),
            roles: Roles::load(configs),
            bar,
            mode: Mode::Desktop,
            show_yazi: false,
            show_rofi: false,
            focus: None,
            rects: Vec::new(),
            headless: false,
        }
    }

    /// A preview that never starts programs (for tests).
    #[cfg(test)]
    pub fn headless(configs: &Path) -> Preview {
        let mut p = Preview::new(configs);
        p.headless = true;
        p
    }

    #[cfg(test)]
    pub fn set_headless(&mut self) {
        self.headless = true;
    }

    /// F12: editor → each preview window → editor.
    pub fn cycle_focus(&mut self) {
        let panes = self.focusable();
        // the right-hand slot is one stop whether it shows the shell or yazi
        let slot = |a: PaneId, b: PaneId| a == b || (a != PaneId::Nvim && b != PaneId::Nvim);
        self.focus = match self
            .focus
            .and_then(|f| panes.iter().position(|p| slot(*p, f)))
        {
            None => panes.first().copied(),
            Some(i) => panes.get(i + 1).copied(),
        };
    }

    /// Y: the right-hand window switches between the shell and yazi.
    pub fn toggle_yazi(&mut self) {
        self.show_yazi = !self.show_yazi;
        if let Some(f) = self.focus
            && f != PaneId::Nvim
        {
            self.focus = Some(if self.show_yazi {
                PaneId::Yazi
            } else {
                PaneId::Shell
            });
        }
    }

    /// True once since the last call if a program printed something.
    /// The folder for the draft colours (and the font samples).
    pub fn draft_dir(&self) -> &Path {
        &self.draft_dir
    }

    /// Set by anything that needs a redraw (programs printing, samples ready).
    pub fn dirty_flag(&self) -> Arc<AtomicBool> {
        self.dirty.clone()
    }

    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    /// Writes the draft palette for nvim (same format as ~/.config/colors/colors.sh).
    pub fn sync_draft(&mut self, p: &Palette) {
        if self.written.as_ref() == Some(p) {
            return;
        }
        let text: String = KEYS
            .iter()
            .map(|k| format!("{k}={}\n", p.get(k).hex()))
            .collect();
        if std::fs::write(&self.draft, text).is_ok() {
            self.written = Some(p.clone());
        }
    }

    pub fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match id {
            PaneId::Nvim => self.nvim.as_mut(),
            PaneId::Shell => self.shell.as_mut(),
            PaneId::Yazi => self.yazi.as_mut(),
        }
    }

    /// Panes that can take the keyboard, in F12 order.
    pub fn focusable(&self) -> Vec<PaneId> {
        self.rects.iter().map(|(id, _)| *id).collect()
    }

    pub fn send_key(&mut self, id: PaneId, k: KeyEvent) {
        if let Some(pane) = self.pane_mut(id) {
            let bytes = key_bytes(k, pane.application_cursor());
            pane.write(&bytes);
        }
    }

    pub fn send_paste(&mut self, id: PaneId, text: &str) {
        if let Some(pane) = self.pane_mut(id) {
            if pane.bracketed_paste() {
                pane.write(format!("\x1b[200~{text}\x1b[201~").as_bytes());
            } else {
                pane.write(text.as_bytes());
            }
        }
    }

    fn command(&self, program: &str) -> CommandBuilder {
        let mut c = CommandBuilder::new(program);
        c.cwd(&self.workdir);
        c.env("TERM", "xterm-256color");
        c.env("COLORTERM", "truecolor");
        c.env("THEME_COLORS", &self.draft);
        c
    }

    /// Starts a pane's program the first time it's shown, at the size it's shown at.
    pub(crate) fn ensure(&mut self, id: PaneId, rows: u16, cols: u16) {
        if self.headless || self.pane_mut(id).is_some() {
            if let Some(p) = self.pane_mut(id) {
                p.resize(rows, cols);
            }
            return;
        }
        let cmd = match id {
            PaneId::Nvim => {
                let mut c = self.command("nvim");
                // read-only, no swap file: it's only there to be looked at
                c.args(["-R", "-n"]);
                c.arg(self.configs.join("nvim/lua/theme.lua"));
                c
            }
            PaneId::Shell => {
                let mut c = self.command("bash");
                c.arg("-i");
                c.env("HISTFILE", "/dev/null"); // the demo commands stay out of your history
                c
            }
            PaneId::Yazi => {
                let mut c = self.command("yazi");
                c.arg(&self.workdir);
                c
            }
        };
        match Pane::spawn(cmd, rows, cols, self.dirty.clone()) {
            Ok(mut pane) => {
                if id == PaneId::Shell {
                    // Typed into the shell like you would: real prompts, real colours.
                    pane.pending_input = Some(
                        b"ls --color=auto\ngit --no-pager log --oneline --graph --decorate -4\nfalse\n"
                            .to_vec(),
                    );
                }
                match id {
                    PaneId::Nvim => self.nvim = Some(pane),
                    PaneId::Shell => self.shell = Some(pane),
                    PaneId::Yazi => self.yazi = Some(pane),
                }
            }
            Err(e) => self.errors.push(format!("{id:?}: {e:#}")),
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.nvim = None;
        self.shell = None;
        self.yazi = None;
        let _ = std::fs::remove_dir_all(&self.draft_dir);
    }
}

/// Removes draft folders of studios that are no longer running (a closed
/// window kills the studio before it can clean up).
fn remove_stale_drafts(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("theme-studio-"))
            .and_then(|n| n.split('-').next())
        else {
            continue;
        };
        let running = pid
            .parse::<u32>()
            .is_ok_and(|p| Path::new(&format!("/proc/{p}")).exists());
        if !running && e.path().join("colors.sh").exists() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_file_matches_colors_sh_format() {
        let dir = std::env::temp_dir().join(format!("theme-studio-prev-{}", std::process::id()));
        let mut prev = Preview::headless(&dir);
        let p = Palette::blank();
        prev.sync_draft(&p);
        let text = std::fs::read_to_string(&prev.draft).unwrap();
        assert_eq!(text.lines().count(), 34);
        assert_eq!(
            text.lines().nth(1),
            Some(format!("bg0={}", p.get("bg0").hex()).as_str())
        );
        let draft_dir = prev.draft_dir.clone();
        drop(prev);
        assert!(!draft_dir.exists(), "draft folder removed on exit");
    }
}
