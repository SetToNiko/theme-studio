//! The Fonts page's preview: the highlighted font drawn before installing
//! it. Google / Fontsource fonts are fetched (one small file) after a short
//! pause on them; Nerd Fonts on request (`p`), since they come as archives.
//! Drawn by ImageMagick + Pango with a fontconfig setup that includes the
//! preview cache (system/download.rs), in the theme's colours.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::fonts::render::SampleRenderer;
use crate::model::color::Rgb;
use crate::model::samples::{self, KINDS, Kind, LANGS};
use crate::system::download::{Entry, Source};

/// How long the highlight rests on a font before its preview is fetched.
const PAUSE: Duration = Duration::from_millis(150);

pub struct FontPreview {
    /// The text shown; None: the font's default (code, icons or prose).
    pub kind: Option<Kind>,
    pub lang: usize,
    /// Prose: the first paragraph shown; code: the first line. And where
    /// the next page starts.
    pub page: usize,
    pub next_page: usize,
    /// Code: lines a page moves by (from the pane's height).
    pub step: usize,
    /// Points.
    pub size: u32,
    /// Fetched fonts: id → the family name to draw with (or what failed).
    pub families: HashMap<String, Result<String, String>>,
    pub fetching: HashSet<String>,
    pub image: Option<StatefulProtocol>,
    pub error: Option<String>,
    renderer: SampleRenderer,
    /// The highlighted font, and since when.
    pub last: Option<String>,
    since: Instant,
    /// The pane's size in cells (from the last draw, once settled).
    pub area: (u16, u16),
    /// The pane's size while resizing (re-rendered once it holds still).
    pub settle: crate::model::settle::Settle,
}

impl FontPreview {
    pub fn new(
        dir: &std::path::Path,
        dirty: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> FontPreview {
        FontPreview {
            kind: None,
            lang: 0,
            page: 0,
            next_page: 0,
            step: 10,
            size: 12,
            families: HashMap::new(),
            fetching: HashSet::new(),
            image: None,
            error: None,
            renderer: SampleRenderer::named(dir, dirty, "preview"),
            last: None,
            since: Instant::now(),
            area: (60, 20),
            settle: Default::default(),
        }
    }

    /// Notes which font is highlighted. A new font starts the pause again
    /// and clears the old image, error and page.
    pub(crate) fn note_highlight(&mut self, id: &str) {
        if self.last.as_deref() != Some(id) {
            self.last = Some(id.to_string());
            self.since = Instant::now();
            self.page = 0;
            self.image = None;
            self.error = None;
        }
    }

    /// True once the highlight has rested on its font for [`PAUSE`].
    pub(crate) fn rested(&self) -> bool {
        self.since.elapsed() >= PAUSE
    }

    /// Asks the renderer for an image (it skips unchanged requests).
    pub(crate) fn request_render(
        &mut self,
        markup: String,
        bg: Rgb,
        dpi: u32,
        fontconfig: Option<PathBuf>,
        wrap_width: Option<u32>,
    ) {
        self.renderer
            .request_with(markup, bg, dpi, fontconfig, wrap_width);
    }

    /// Picks up a finished image (or its error) from the renderer. True
    /// when something changed.
    pub(crate) fn finish_render(&mut self, picker: Option<&Picker>) -> bool {
        match self.renderer.take() {
            Some(Ok(img)) => {
                self.image = picker.map(|p| p.new_resize_protocol(img));
                self.error = None;
                true
            }
            Some(Err(err)) => {
                self.error = Some(err);
                true
            }
            None => false,
        }
    }

    /// The text for a font: the chosen one, else its default.
    pub fn kind_for(&self, e: &Entry) -> Kind {
        self.kind.unwrap_or_else(|| {
            samples::default_kind(&e.category, matches!(e.source, Source::Nerd { .. }))
        })
    }
}

/// The choices of the preview's title dropdowns: 0 the text kind, 1 the
/// language.
pub fn menu_items(menu: u8) -> Vec<&'static str> {
    if menu == 0 {
        KINDS.iter().map(|(_, l)| kind_label(l)).collect()
    } else {
        LANGS.iter().map(|l| l.0).collect()
    }
}

/// A text kind as the dropdown shows it ("Code", "README").
pub fn kind_label(l: &str) -> &'static str {
    match l {
        "code" => "Code",
        "icons" => "Icons",
        "prose" => "Prose",
        _ => "README",
    }
}
