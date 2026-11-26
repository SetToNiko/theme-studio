//! App's methods for the Tournament tab's rounds and results: drawing the
//! two fonts, the keys while playing and on the results, installing or using
//! a result, and clicks.
//!
//! This is another `impl App { ... }` block (see `fonts_preview_actions.rs`
//! for the same style). The set-up and fetching are in `tour_actions.rs`;
//! `fonts/tournament_page.rs` keeps the tab's own data and the bracket is in `model/tournament.rs`.

use crate::app::App;
use crate::fonts::tournament_page::Stage;
use crate::keymap::action::Action;
use crate::model::samples::{self, Kind};
use crate::model::tournament::{self, Tournament};
use crate::system::download;

impl App {
    /// Asks pane `i` to show entrant `who`, and picks up a finished image.
    pub(in crate::app) fn tour_draw_pane(&mut self, i: usize, who: Option<usize>) -> bool {
        let Some(who) = who else {
            let p = &mut self.tour().panes[i];
            let had = p.image.is_some();
            p.image = None;
            p.shows = None;
            return had;
        };
        let Some(e) = self.tour().t.as_ref().map(|t| t.entrants[who].clone()) else {
            return false;
        };
        let family = match self.pv().families.get(&e.id) {
            Some(Ok(f)) => f.clone(),
            _ => return false,
        };
        let cell = self.picker.as_ref().map_or((8, 16), |p| {
            let s = p.font_size();
            (u32::from(s.width), u32::from(s.height))
        });
        self.request_pane_render(i, who, &e, &family, cell);
        self.take_pane_image(i)
    }

    /// Sizes the sample text for pane `i` (a terminal cell is `cell` pixels
    /// wide and high) and asks the renderer for entrant `who`'s image.
    fn request_pane_render(
        &mut self,
        i: usize,
        who: usize,
        e: &tournament::Entrant,
        family: &str,
        (cell_w, cell_h): (u32, u32),
    ) {
        let dpi = self.fonts.dpi;
        let palette = self.editor.palette.clone();
        let bg = palette.get("bg0");
        let t = self.tour();
        if t.panes[i].shows != Some(who) {
            t.panes[i].shows = Some(who);
            t.panes[i].image = None;
        }
        let (cols, rows) = (u32::from(t.panes[i].area.0), u32::from(t.panes[i].area.1));
        let width = (cols * cell_w).saturating_sub(24).max(100);
        let st = samples::Style {
            family,
            size: t.pt,
            palette: &palette,
        };
        let px = (t.pt * dpi / 72).max(6);
        let lines = ((rows * cell_h) / (px * 12 / 10).max(1)) as usize;
        let markup = sample_text(&st, t.kind, t.lang, (width, px, lines));
        let fontconfig = (!e.installed).then(|| download::preview::preview_fontconfig_for(&e.id));
        let wrap = (t.kind == Kind::Prose).then_some(width);
        t.panes[i]
            .renderer
            .request_with(markup, bg, dpi, fontconfig, wrap);
    }

    /// Picks up pane `i`'s finished image (or error). True when it changed.
    fn take_pane_image(&mut self, i: usize) -> bool {
        let p = &mut self.tour().panes[i];
        match p.renderer.take() {
            Some(Ok(img)) => {
                let proto = self.picker.as_ref().map(|pk| pk.new_resize_protocol(img));
                let p = &mut self.tour().panes[i];
                p.image = proto;
                p.error = None;
                true
            }
            Some(Err(err)) => {
                self.tour().panes[i].error = Some(err);
                true
            }
            None => false,
        }
    }

    pub(in crate::app) fn tour_save(&mut self) {
        let dir = tournament::dir();
        if let Some(t) = &self.tour().t
            && let Err(e) = t.save(&dir)
        {
            let msg = format!("{e:#}");
            self.messages.error(msg);
        }
    }

    /// Keys on the Tournament tab. True if handled.
    pub fn run_tournament(&mut self, a: Action) -> bool {
        if self.tour().bracket {
            let t = self.tour();
            match a {
                Action::Down => t.bracket_scroll += 1,
                Action::Up => t.bracket_scroll = t.bracket_scroll.saturating_sub(1),
                Action::PageDown => t.bracket_scroll += 10,
                Action::PageUp => t.bracket_scroll = t.bracket_scroll.saturating_sub(10),
                Action::Back | Action::TourBracket | Action::Confirm => t.bracket = false,
                _ => {}
            }
            return true;
        }
        match self.tour().stage {
            Stage::Setup => self.run_tour_setup(a),
            Stage::Fetching => match a {
                Action::Back => {
                    let t = self.tour();
                    t.stage = Stage::Setup;
                    t.pending.clear();
                    self.prefetch.urgent.clear();
                    true
                }
                _ => false,
            },
            Stage::Playing => self.run_tour_playing(a),
            Stage::Results => self.run_tour_results(a),
        }
    }

    fn run_tour_playing(&mut self, a: Action) -> bool {
        if self.tour().t.is_none() {
            return false;
        }
        if self.tour_view_key(a) {
            return true;
        }
        let t = self.tour();
        match a {
            Action::Left | Action::Right => self.tour_pick(a == Action::Left),
            Action::TourUndo => {
                if t.t.as_mut().is_some_and(Tournament::undo) {
                    self.tour_save();
                } else {
                    self.messages.info("No pick to take back");
                }
            }
            Action::TourNames => t.names = !t.names,
            Action::Back => {
                t.stage = Stage::Setup;
                t.past = None;
                self.messages.info("Saved: pick it up again from the list");
            }
            _ => return false,
        }
        true
    }

    /// Picks the left (`left`) or right font. After the last pick the results
    /// page opens.
    fn tour_pick(&mut self, left: bool) {
        let t = self.tour();
        if t.pick(left) {
            t.stage = Stage::Results;
            t.sel = 0;
            t.names = true;
        }
        t.past = None;
        self.tour_save();
        if self.tour().stage == Stage::Results {
            self.messages.info("The tournament is over: the top 10");
        }
    }

    /// Keys that change how the fonts are shown, while playing and on the
    /// results page: the bracket, text kind, language and size. True when `a`
    /// was one of them.
    fn tour_view_key(&mut self, a: Action) -> bool {
        let t = self.tour();
        match a {
            Action::TourBracket => {
                t.bracket = true;
                t.bracket_scroll = 0;
            }
            Action::PreviewText => {
                t.kind = if t.kind == Kind::Prose {
                    Kind::Code
                } else {
                    Kind::Prose
                }
            }
            Action::PreviewLang => t.lang = (t.lang + 1) % samples::langs::LANGS.len(),
            Action::SizeUp => t.pt = (t.pt + 1).min(48),
            Action::SizeDown => t.pt = t.pt.saturating_sub(1).max(6),
            _ => return false,
        }
        true
    }

    fn run_tour_results(&mut self, a: Action) -> bool {
        if self.tour().t.is_none() {
            return false;
        }
        if self.tour_view_key(a) || self.results_pick_key(a) {
            return true;
        }
        let last = self.results_top().len().saturating_sub(1);
        let t = self.tour();
        match a {
            Action::Up => t.sel = t.sel.saturating_sub(1),
            Action::Down => t.sel = (t.sel + 1).min(last),
            Action::TourUndo => {
                // back into the final
                if let Some(x) = t.t.as_mut()
                    && x.undo()
                {
                    t.stage = Stage::Playing;
                    self.tour_save();
                }
            }
            Action::Back => {
                t.stage = Stage::Setup;
                t.past = None;
            }
            _ => return false,
        }
        true
    }

    /// The entrants of the results list (the top 10), best first.
    fn results_top(&mut self) -> Vec<usize> {
        let Some(x) = self.tour().t.as_ref() else {
            return Vec::new();
        };
        x.top().into_iter().flat_map(|(_, v)| v).collect()
    }

    /// Keys that act on the highlighted result: install it, use it for a
    /// role, or fill the family roles. True when `a` was one of them.
    fn results_pick_key(&mut self, a: Action) -> bool {
        let top = self.results_top();
        let who = top.get(self.tour().sel).copied();
        match a {
            Action::Confirm => {
                if let Some(w) = who {
                    self.tour_install(w);
                }
            }
            Action::UseFor(r) => {
                if let Some(w) = who {
                    self.tour_use(w, &[r as usize]);
                }
            }
            Action::FillFamily => {
                if let Some(w) = who {
                    self.tour_use(w, &[0, 1, 2]);
                }
            }
            _ => return false,
        }
        true
    }

    /// Whether a results font is installed (by its name, or the family its
    /// preview was drawn with).
    fn tour_installed(&mut self, who: usize) -> Option<String> {
        let e = self.tour().t.as_ref()?.entrants[who].clone();
        let family = match self.pv().families.get(&e.id) {
            Some(Ok(f)) => f.clone(),
            _ => e.name.clone(),
        };
        let norm = |s: &str| s.to_lowercase().replace(' ', "");
        self.fonts
            .installed
            .iter()
            .find(|f| {
                norm(&f.family).starts_with(&norm(&family)) || norm(&f.family) == norm(&e.name)
            })
            .map(|_| family)
    }

    /// Enter on a result: installs it (catalogue fonts).
    fn tour_install(&mut self, who: usize) {
        if self.tour_installed(who).is_some() {
            return self
                .messages
                .info("Installed already: 1, 2 or 3 use it, F fills every role");
        }
        let Some(id) = self.tour().t.as_ref().map(|t| t.entrants[who].id.clone()) else {
            return;
        };
        let entry = self
            .dl
            .catalogue
            .iter()
            .flat_map(|c| c.google.iter().chain(c.nerd.iter()))
            .find(|e| e.id == id)
            .cloned();
        match entry {
            Some(e) => self.dl_install_entry(e),
            None => self.messages.info("The catalogue is still loading"),
        }
    }

    /// 1 / 2 / 3 or F on a result: uses it for roles (applied at once).
    fn tour_use(&mut self, who: usize, roles: &[usize]) {
        let Some(family) = self.tour_installed(who) else {
            return self.messages.info("Install it first (Enter)");
        };
        let fill = self.fonts.fill_roles(&family, roles);
        self.after_fill(fill);
    }

    /// A click on a pane while playing: picks that font.
    pub fn tour_click_pane(&mut self, i: u8) {
        if self.tour().stage == Stage::Playing {
            self.run_tour_playing(if i == 0 { Action::Left } else { Action::Right });
        }
    }

    /// A click on a row: setup rows and results move the cursor; a second
    /// click acts like Enter.
    pub fn tour_click_row(&mut self, row: usize) {
        let t = self.tour();
        if t.sel == row {
            self.run_tournament(Action::Confirm);
        } else {
            t.sel = row;
        }
    }
}

/// The sample text of one pane: prose or code in `st`'s font. `(width, px,
/// lines)` are the pane's width in pixels, the line height in pixels and the
/// number of lines that fit.
fn sample_text(st: &samples::Style, kind: Kind, lang: usize, size: (u32, u32, usize)) -> String {
    let (width, px, lines) = size;
    match kind {
        Kind::Prose => {
            let per_line = width / (px * 55 / 100).max(1);
            samples::prose_page(st, 0, (per_line as usize * lines * 85 / 100).max(200)).0
        }
        _ => samples::code(st, lang, 0, lines + 1),
    }
}
