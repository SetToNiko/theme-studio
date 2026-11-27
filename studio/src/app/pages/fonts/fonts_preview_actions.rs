//! App's methods for the Fonts page's preview: fetching and drawing the
//! highlighted font, its keys, the wheel, and the title dropdowns.
//!
//! This is another `impl App { ... }` block (see `fonts_dl_actions.rs` for
//! the same style). The preview's own data (`FontPreview`, its text kinds,
//! pause and renderer) and the dropdown lists stay in `fonts/preview_page.rs`;
//! this file only holds the methods that need the whole `App`.

use crate::app::app_state::App;
use crate::app::state::nav::{Overlay, Page};
use crate::fonts::pages::download::{DlRow, DlTab, mb};
use crate::fonts::pages::preview::FontPreview;
use crate::keymap::action::Action;
use crate::model::samples::langs::LANGS;
use crate::model::samples::{self, KINDS, Kind};
use crate::system::download;
use crate::system::download::types::{Entry, Source};

impl App {
    pub(crate) fn pv(&mut self) -> &mut FontPreview {
        if self.dl.preview.is_none() {
            self.dl.preview = Some(FontPreview::new(
                self.preview.draft_dir(),
                self.preview.dirty_flag(),
            ));
        }
        self.dl.preview.as_mut().unwrap()
    }

    /// The font the preview shows: the highlighted catalogue font, or on the
    /// Installed tab the highlighted installed one (drawn from its files).
    pub fn preview_entry(&mut self) -> Option<Entry> {
        if let Some(e) = self.dl_selected() {
            return Some(e);
        }
        let Some(DlRow::Listed(name)) = self.dl_rows().get(self.dl.sel).cloned() else {
            return None;
        };
        let listed = self.dl_listed();
        let v = listed.get(&name)?;
        let id = v.get("id")?.as_str()?.to_string();
        let nerd = v.get("source").and_then(|s| s.as_str()) == Some("nerd");
        let category = self.catalogue_category(&id, nerd);
        Some(listed_entry(name, id, category, nerd))
    }

    /// The catalogue's category of font `id`, for the default text. Without
    /// the catalogue, Nerd Fonts (`nerd`) get icons and the rest code.
    fn catalogue_category(&self, id: &str, nerd: bool) -> String {
        self.dl
            .catalogue
            .as_ref()
            .and_then(|c| c.google.iter().chain(c.nerd.iter()).find(|e| e.id == id))
            .map_or_else(
                || if nerd { "nerd font" } else { "monospace" }.into(),
                |e| e.category.clone(),
            )
    }

    /// Each loop: fetch the highlighted font after a pause, render it, pick
    /// up a finished image. True when something changed.
    pub fn poll_preview(&mut self) -> bool {
        if self.page != Page::Downloads || !self.layout.fonts_preview_visible {
            return false;
        }
        let Some(e) = self.preview_entry() else {
            return false;
        };
        self.start_preview_fetch(&e);
        let Some(Ok(family)) = self.pv().families.get(&e.id).cloned() else {
            return false;
        };
        self.render_preview(&e, &family);
        // the preview exists here (`self.pv()` made it above)
        let picker = self.picker.as_ref(); // a field of its own, so `dl` can be borrowed too
        let Some(pv) = self.dl.preview.as_mut() else {
            return false;
        };
        pv.finish_render(picker)
    }

    /// Notes the highlighted font `e` and starts the background work that gets
    /// its family: a scan of its files (installed) or a fetch (after a pause).
    fn start_preview_fetch(&mut self, e: &Entry) {
        let installed = self.dl.tab == DlTab::Installed;
        let pv = self.pv();
        pv.note_highlight(&e.id);
        // installed: its family from its own files, nothing to fetch
        if installed && !pv.families.contains_key(&e.id) && pv.fetching.insert(e.id.clone()) {
            download::preview::scan_installed(e.id.clone(), self.dl_sender());
        }
        let pv = self.pv();
        let nerd = matches!(e.source, Source::Nerd { .. });
        if !nerd
            && !installed
            && pv.rested()
            && !pv.families.contains_key(&e.id)
            && !pv.fetching.contains(&e.id)
        {
            pv.fetching.insert(e.id.clone());
            download::preview::fetch_preview(e.clone(), self.dl_sender());
        }
    }

    /// Asks for the preview image (the renderer skips unchanged requests).
    fn render_preview(&mut self, e: &Entry, family: &str) {
        // installed fonts are in the system's fontconfig; fetched ones get a
        // setup of their own
        let installed = self.dl.tab == DlTab::Installed;
        let (cell_w, cell_h) = self.picker.as_ref().map_or((8, 16), |p| {
            let s = p.font_size();
            (u32::from(s.width), u32::from(s.height))
        });
        let dpi = self.fonts.dpi;
        let palette = self.editor.palette.clone();
        let bg = palette.get("bg0");
        let pv = self.pv();
        let kind = pv.kind_for(e);
        let (cols, rows) = (u32::from(pv.area.0), u32::from(pv.area.1));
        let width = (cols * cell_w).saturating_sub(24).max(100);
        let st = samples::Style {
            family,
            size: pv.size,
            palette: &palette,
        };
        let markup = sample_markup(pv, kind, &st, dpi, (rows * cell_h, width));
        pv.request_render(
            markup,
            bg,
            dpi,
            (!installed).then(|| download::preview::preview_fontconfig_for(&e.id)),
            // prose and the README wrap; code and icons are cut, like a terminal
            matches!(kind, Kind::Prose | Kind::Readme).then_some(width),
        );
    }

    /// Keys for the preview (on the Fonts page). True if handled.
    // Long function (reason verified, phase 6): one match over the preview's key actions.
    pub fn preview_action(&mut self, a: Action) -> bool {
        let sel = self.preview_entry();
        let pv = self.pv();
        match a {
            Action::PreviewText => {
                let cur = sel.as_ref().map_or(Kind::Code, |e| pv.kind_for(e));
                let i = KINDS.iter().position(|(k, _)| *k == cur).unwrap_or(0);
                pv.kind = Some(KINDS[(i + 1) % KINDS.len()].0);
                pv.page = 0;
            }
            Action::PreviewLang => {
                pv.lang = (pv.lang + 1) % LANGS.len();
                pv.page = 0;
            }
            Action::PreviewNext => {
                let code = sel.as_ref().is_some_and(|e| pv.kind_for(e) == Kind::Code);
                pv.page = if code {
                    (pv.page + pv.step).min(samples::code_lines(pv.lang).saturating_sub(1))
                } else {
                    pv.next_page.max(pv.page)
                };
            }
            Action::PreviewPrev => {
                let code = sel.as_ref().is_some_and(|e| pv.kind_for(e) == Kind::Code);
                // prose goes back a few paragraphs (pages aren't kept)
                pv.page = pv.page.saturating_sub(if code { pv.step } else { 4 });
            }
            Action::SizeUp => pv.size = (pv.size + 1).min(48),
            Action::SizeDown => pv.size = pv.size.saturating_sub(1).max(6),
            Action::PreviewFetch => match sel {
                Some(e) if !pv.families.contains_key(&e.id) && !pv.fetching.contains(&e.id) => {
                    pv.fetching.insert(e.id.clone());
                    let what = match &e.source {
                        Source::Nerd { size, .. } => format!(" ({})", mb(*size)),
                        _ => String::new(),
                    };
                    let msg = format!("Fetching {} for its preview{what}…", e.family);
                    download::preview::fetch_preview(e, self.dl_sender());
                    self.messages.info(msg);
                }
                _ => {}
            },
            _ => return false,
        }
        true
    }

    /// The wheel over the preview: code moves three lines, prose a page.
    pub fn preview_scroll(&mut self, d: i32) {
        let code = self
            .preview_entry()
            .is_some_and(|e| self.pv().kind_for(&e) == Kind::Code);
        if !code {
            self.preview_action(if d > 0 {
                Action::PreviewNext
            } else {
                Action::PreviewPrev
            });
            return;
        }
        let pv = self.pv();
        let last = samples::code_lines(pv.lang).saturating_sub(1);
        pv.page = if d > 0 {
            (pv.page + 3).min(last)
        } else {
            pv.page.saturating_sub(3)
        };
    }

    /// The text kind chosen from the title's dropdown.
    pub fn set_preview_kind(&mut self, kind: Kind) {
        let pv = self.pv();
        pv.kind = Some(kind);
        pv.page = 0;
    }

    /// The language chosen from the title's dropdown.
    pub fn set_preview_lang(&mut self, lang: usize) {
        let pv = self.pv();
        pv.lang = lang % LANGS.len();
        pv.page = 0;
    }

    /// What the preview pane says when there's no image.
    pub fn preview_note(&mut self) -> String {
        let Some(e) = self.preview_entry() else {
            return String::new();
        };
        let pv = self.pv();
        if let Some(Err(err)) = pv.families.get(&e.id) {
            return format!("Couldn't fetch it: {err}");
        }
        if let Some(err) = &pv.error {
            return format!("Couldn't draw it: {err}");
        }
        if pv.fetching.contains(&e.id) {
            return "fetching the font…".into();
        }
        match &e.source {
            Source::Nerd { size, .. } if !pv.families.contains_key(&e.id) => {
                format!("p fetches it for a preview ({})", mb(*size))
            }
            _ => "drawing…".into(),
        }
    }

    /// Opens a title dropdown under `at`, on the current choice.
    pub fn open_menu(&mut self, menu: u8, at: ratatui::layout::Rect) {
        let sel = self.preview_entry();
        let pv = self.pv();
        let sel = if menu == 0 {
            let cur = sel.map_or(Kind::Code, |e| pv.kind_for(&e));
            KINDS.iter().position(|(k, _)| *k == cur).unwrap_or(0)
        } else {
            pv.lang
        };
        self.overlay = Overlay::Menu { menu, sel, at };
    }

    /// A choice from a title dropdown.
    pub fn choose_menu(&mut self, menu: u8, i: usize) {
        self.overlay = Overlay::None;
        if menu == 0 {
            self.set_preview_kind(KINDS[i % KINDS.len()].0);
        } else {
            self.set_preview_lang(i);
        }
    }
}

/// Builds the sample text (as markup) for the preview's text kind. Code and
/// prose also set the page size (`step`, `next_page`) that the paging keys
/// use. `(height, width)` is the preview area in pixels.
fn sample_markup(
    pv: &mut FontPreview,
    kind: Kind,
    st: &samples::Style,
    dpi: u32,
    (height, width): (u32, u32),
) -> String {
    let px = (pv.size * dpi / 72).max(6);
    match kind {
        Kind::Code => {
            // about as many lines as fit, less two kept from the last page
            let lines = (height / (px * 12 / 10).max(1)) as usize;
            pv.step = lines.saturating_sub(2).max(1);
            let last = samples::code_lines(pv.lang).saturating_sub(1);
            pv.page = pv.page.min(last);
            // only what shows (one more for a part-shown line)
            samples::code(st, pv.lang, pv.page, lines + 1)
        }
        Kind::Icons => samples::icons(st),
        Kind::Readme => samples::readme(st),
        Kind::Prose => {
            // about as many characters as fit
            let per_line = width / (px * 55 / 100).max(1);
            let lines = height / (px * 16 / 10).max(1);
            let (m, next) =
                samples::prose_page(st, pv.page, (per_line * lines * 85 / 100) as usize);
            pv.next_page = next;
            m
        }
    }
}

/// The `Entry` of an installed font `name` (its catalogue `id` and `category`;
/// `nerd` when it comes from Nerd Fonts), with the plain values of a font that
/// is not in the catalogue.
fn listed_entry(name: String, id: String, category: String, nerd: bool) -> Entry {
    Entry {
        id,
        family: name.clone(),
        category,
        license: String::new(),
        weights: vec![400],
        italic: false,
        variable: false,
        source: if nerd {
            Source::Nerd {
                asset: name,
                url: String::new(),
                size: 0,
            }
        } else {
            Source::Fontsource
        },
        subset: "latin".into(),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::startup::Start;
    use crate::app::state::nav::Page;
    use crate::app::test_support::*;
    use std::time::Duration;

    #[test]
    fn installed_fonts_preview_from_their_files() {
        // a real font file to stand in (skipped without one, e.g. in the
        // Nix build's sandbox)
        let Some(file) = std::process::Command::new("fc-match")
            .args(["-f", "%{file}", "monospace"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .filter(|f| has_font_extension(f))
        else {
            eprintln!("skipped: no font file");
            return;
        };
        let store = temp_store("installed-preview");
        let dir = store.dir.clone();
        let mut app = test_app(store, Start::Named("One".into()));
        let fonts = crate::model::paths::installed_fonts_dir().join("test-mono");
        std::fs::create_dir_all(&fonts).unwrap();
        let ext = if has_extension(&file, "otf") {
            "otf"
        } else {
            "ttf"
        };
        std::fs::copy(&file, fonts.join(format!("TestMono-Regular.{ext}"))).unwrap();
        let list = app.installed_list_path();
        std::fs::create_dir_all(list.parent().unwrap()).unwrap();
        std::fs::write(
        &list,
        r#"{"version": 1, "fonts": {"Test Mono": {"source": "fontsource", "id": "test-mono"}}}"#,
    )
    .unwrap();
        app.page = Page::Downloads;
        app.dl.tab = crate::fonts::pages::download::DlTab::Installed;
        app.dl.listed = None;
        app.dl.sel = 0;
        // the highlighted installed font is the preview's, with code by default
        let e = app.preview_entry().unwrap();
        assert_eq!(
            (e.id.as_str(), e.category.as_str()),
            ("test-mono", "monospace")
        );
        // its family comes from its files, without fetching anything
        let start = std::time::Instant::now();
        loop {
            app.poll_downloads();
            if let Some(r) = app
                .dl
                .preview
                .as_ref()
                .and_then(|p| p.families.get("test-mono"))
            {
                assert!(r.as_ref().is_ok_and(|f| !f.is_empty()), "{r:?}");
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "no family name");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_dir_all(fonts).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// True when `file` ends in `.ext` (letter case does not matter).
    fn has_extension(file: &str, ext: &str) -> bool {
        std::path::Path::new(file)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    }

    /// True for a font file that the preview can read (`.ttf` or `.otf`).
    fn has_font_extension(file: &str) -> bool {
        has_extension(file, "ttf") || has_extension(file, "otf")
    }
}
