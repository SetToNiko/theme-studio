//! The studio's settings data: the structs saved in settings.json, their
//! defaults, and the sizes and constants they use. Loading and saving live in
//! the parent module (`config`).

use serde::{Deserialize, Serialize};

use crate::fonts::{Filters, NerdKind, Spacing};

/// How the font list starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FontListSettings {
    pub hide_bitmap: bool,
    pub hide_cjk_emoji_math: bool,
    /// "any", "mono" or "propo"
    pub spacing: String,
    /// "any", "nerd-font", "nerd-font-mono", "nerd-font-propo" or "no-nerd-font"
    pub nerd_font: String,
}

impl Default for FontListSettings {
    fn default() -> Self {
        FontListSettings {
            hide_bitmap: true,
            hide_cjk_emoji_math: true,
            spacing: "any".into(),
            nerd_font: "any".into(),
        }
    }
}

impl FontListSettings {
    pub fn filters(&self) -> Filters {
        Filters {
            spacing: match self.spacing.as_str() {
                "mono" => Spacing::Mono,
                "propo" => Spacing::Propo,
                _ => Spacing::Any,
            },
            nerd: match self.nerd_font.as_str() {
                "nerd-font" => NerdKind::Nerd,
                "nerd-font-mono" => NerdKind::NerdMono,
                "nerd-font-propo" => NerdKind::NerdPropo,
                "no-nerd-font" => NerdKind::NotNerd,
                _ => NerdKind::Any,
            },
            show_bitmap: !self.hide_bitmap,
            show_special: !self.hide_cjk_emoji_math,
            need: None,
        }
    }
}

/// How the colour column starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ColourSettings {
    /// Groups open at start, by name ("Base", "Backgrounds", …).
    pub open_groups: Vec<String>,
    /// "hidden" or "shown"
    pub picker_at_start: String,
    /// "hide-on-esc" or "stay-open"
    pub picker_after_editing: String,
    /// How the selected colour's derivation relatives are marked.
    pub derived_highlight: LookSettings,
}

impl Default for ColourSettings {
    fn default() -> Self {
        ColourSettings {
            open_groups: vec!["Base".into()],
            picker_at_start: "hidden".into(),
            picker_after_editing: "hide-on-esc".into(),
            derived_highlight: LookSettings::derived_default(),
        }
    }
}

impl ColourSettings {
    pub fn picker_shown(&self) -> bool {
        self.picker_at_start == "shown"
    }

    pub fn picker_hides_on_esc(&self) -> bool {
        self.picker_after_editing != "stay-open"
    }
}

/// How the fonts column starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FontSettings {
    /// "shown" or "hidden"
    pub sample_at_start: String,
    /// Opening the font list unfolds the groups of the role's current font.
    pub list_opens_to_current_font: bool,
    /// The list of downloaded fonts that packaging/nix/fonts.nix reads ("": next to
    /// settings.json; "~/" works).
    pub installed_list: String,
    // (the per-role filters, mono-filter / ui-filter / bar-filter, went: each
    // role now has a fixed requirement, fonts_pick.rs)
}

impl Default for FontSettings {
    fn default() -> Self {
        FontSettings {
            sample_at_start: "shown".into(),
            list_opens_to_current_font: true,
            installed_list: String::new(),
        }
    }
}

/// The Fonts (download) page: fetching previews ahead of time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FontsPageSettings {
    /// Fetch previews in the background (a few at a time).
    pub prefetch: bool,
    /// Which: "monospace", "sans-serif", "serif", "display", "handwriting",
    /// "nerd font".
    pub prefetch_categories: Vec<String>,
}

impl Default for FontsPageSettings {
    fn default() -> Self {
        FontsPageSettings {
            prefetch: false,
            prefetch_categories: vec!["monospace".into()],
        }
    }
}

/// The categories previews can be prefetched for: (catalogue category, label).
pub const PREFETCH_CATEGORIES: [(&str, &str); 6] = [
    ("monospace", "Google monospace"),
    ("sans-serif", "Google sans-serif"),
    ("serif", "Google serif"),
    ("display", "Google display"),
    ("handwriting", "Google handwriting"),
    ("nerd font", "Nerd Fonts"),
];

impl FontsPageSettings {
    pub fn prefetches(&self, category: &str) -> bool {
        self.prefetch_categories.iter().any(|c| c == category)
    }
}

/// Pane sizes, as shares of the space they divide (minimum sizes still
/// apply). Resizing changes them while the studio runs; these are where it
/// starts and what `=` in resize mode goes back to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Sizes {
    /// Three columns: the fonts column's share of the width.
    pub fonts_width: f64,
    /// Three columns: the colour column's share of the width.
    pub colours_width: f64,
    /// The font sample's share of the fonts column.
    pub sample_height: f64,
    /// The picker's share of the colour column; 0 = automatic (it grows when
    /// the whole colour list fits).
    pub picker_height: f64,
    /// nvim's share of the preview (the shell gets the rest).
    pub preview_split: f64,
    /// Narrow layout: the preview row's share of the height.
    pub narrow_preview_height: f64,
    /// Narrow layout: the fonts column's share of the top row.
    pub narrow_fonts_width: f64,
}

impl Default for Sizes {
    fn default() -> Self {
        Sizes {
            fonts_width: 0.28,
            colours_width: 0.32,
            sample_height: 0.3,
            picker_height: 0.0,
            preview_split: 0.5,
            narrow_preview_height: 0.42,
            narrow_fonts_width: 0.45,
        }
    }
}

/// Where things go on screen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LayoutSettings {
    /// The page tabs: "bottom" (in the status line) or "top" (their own row).
    pub tabs: String,
    /// Tabs inside a page (the Settings page's): "top" or "bottom".
    pub inner_tabs: String,
    /// The key hints in the status line: "shown" or "hidden".
    pub key_hints: String,
    /// The previews at start: the Main page's (nvim and the shell) and the
    /// Fonts page's, "shown" or "hidden".
    pub main_preview: String,
    pub fonts_preview: String,
    pub sizes: Sizes,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        LayoutSettings {
            tabs: "bottom".into(),
            inner_tabs: "top".into(),
            key_hints: "shown".into(),
            main_preview: "shown".into(),
            fonts_preview: "shown".into(),
            sizes: Sizes::default(),
        }
    }
}

impl LayoutSettings {
    pub fn tabs_on_top(&self) -> bool {
        self.tabs == "top"
    }

    pub fn inner_tabs_on_top(&self) -> bool {
        self.inner_tabs != "bottom"
    }
}

/// settings.json: how things look when the studio starts.
/// (`version` is the file format, for migrating later versions.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    pub version: u32,
    pub layout: LayoutSettings,
    pub colours: ColourSettings,
    pub fonts: FontSettings,
    pub font_list: FontListSettings,
    pub hover: HoverSettings,
    pub messages: MessageSettings,
    pub fonts_page: FontsPageSettings,
}

/// How messages show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MessageSettings {
    /// Seconds a message stays in the status line; 0 = until the next key.
    /// (Errors always stay until the next key.)
    pub seconds: u32,
    /// Save confirmations: "bar" (the status line) or "popup" (a box that
    /// stays until a key or a click closes it).
    pub saved: String,
}

impl Default for MessageSettings {
    fn default() -> Self {
        MessageSettings {
            seconds: 3,
            saved: "bar".into(),
        }
    }
}

impl MessageSettings {
    /// The choices for `seconds`.
    pub const SECONDS: [u32; 6] = [1, 2, 3, 5, 10, 0];

    pub fn saved_in_popup(&self) -> bool {
        self.saved == "popup"
    }
}

/// A highlight look: any mix of a background, an underline and a text
/// colour. Colours are keys of the active system palette ("yellow", "bg3").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LookSettings {
    /// A background colour.
    pub highlight: bool,
    pub highlight_colour: String,
    /// "off", "single" or "double"
    pub underline: String,
    pub underline_colour: String,
    /// A text colour and weight.
    pub text: bool,
    pub text_colour: String,
    /// "normal", "bold" or "dim"
    pub text_weight: String,
}

impl Default for LookSettings {
    fn default() -> Self {
        LookSettings {
            highlight: false,
            highlight_colour: "bg3".into(),
            underline: "single".into(),
            underline_colour: "yellow".into(),
            text: true,
            text_colour: "yellow".into(),
            text_weight: "normal".into(),
        }
    }
}

impl LookSettings {
    /// The derived-colour highlight's default: a background only.
    pub fn derived_default() -> Self {
        LookSettings {
            highlight: true,
            highlight_colour: "bg_visual_blue".into(),
            underline: "off".into(),
            text: false,
            ..Default::default()
        }
    }

    /// Underline styles that are no longer offered become single.
    fn normalise(&mut self) {
        if !["off", "single", "double"].contains(&self.underline.as_str()) {
            self.underline = "single".into();
        }
    }
}

/// How clickable things look under the mouse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct HoverSettings {
    #[serde(flatten)]
    pub look: LookSettings,
    /// The border of a pane under the mouse.
    pub pane_border: String,
    /// The frame of a preview window (nvim, the shell) under the mouse.
    pub preview_border: String,
}

impl Default for HoverSettings {
    fn default() -> Self {
        HoverSettings {
            look: LookSettings::default(),
            pane_border: "aqua".into(),
            preview_border: "purple".into(),
        }
    }
}

impl Settings {
    /// Values from older versions that are no longer offered.
    pub fn normalise(&mut self) {
        self.hover.look.normalise();
        self.colours.derived_highlight.normalise();
        if !["top", "bottom"].contains(&self.layout.inner_tabs.as_str()) {
            self.layout.inner_tabs = "top".into();
        }
        if !["bar", "popup"].contains(&self.messages.saved.as_str()) {
            self.messages.saved = "bar".into();
        }
    }
}

/// settings.json's format version.
pub const SETTINGS_VERSION: u32 = 1;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            layout: LayoutSettings::default(),
            colours: ColourSettings::default(),
            fonts: FontSettings::default(),
            font_list: FontListSettings::default(),
            hover: HoverSettings::default(),
            messages: MessageSettings::default(),
            fonts_page: FontsPageSettings::default(),
        }
    }
}
