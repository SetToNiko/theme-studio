//! The types that the download files share: fonts, catalogues and the
//! messages of the background threads.

use serde_json::Value;

/// Where a font comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// In the google/fonts repo (license folder guessed from `license`).
    Google,
    /// Fontsource's own files (fonts not from Google).
    Fontsource,
    /// A Nerd Fonts release asset: `<asset>.tar.xz`.
    Nerd {
        asset: String,
        url: String,
        size: u64,
    },
}

/// A font that can be downloaded.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub family: String,
    /// sans-serif, serif, monospace, display, handwriting; "nerd font".
    pub category: String,
    pub license: String,
    pub weights: Vec<u64>,
    pub italic: bool,
    pub variable: bool,
    pub source: Source,
    /// Fontsource's default subset ("latin"; the preview file's).
    pub subset: String,
}

/// The two catalogues.
#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    pub google: Vec<Entry>,
    pub nerd: Vec<Entry>,
    pub nerd_tag: String,
}

/// What a background thread reports.
#[derive(Debug)]
pub enum Msg {
    Catalogue(Result<Catalogue, String>),
    /// Bytes so far of the total (0 if unknown).
    Progress {
        id: String,
        done: u64,
        total: u64,
    },
    /// An install finished: the list entry to record (name, value).
    /// `warning` is set when the files are installed but a later step
    /// (the fontconfig cache refresh) failed: the font still works.
    Installed {
        id: String,
        result: Result<(String, Value), String>,
        warning: Option<String>,
    },
    /// A preview font is ready: the family name to draw it with.
    Preview {
        id: String,
        result: Result<String, String>,
    },
    /// Prefetching's queue, made on a thread: for which categories, the
    /// fonts still to fetch, and how many there are in all.
    PrefetchQueue {
        categories: Vec<String>,
        queue: Vec<Entry>,
        total: usize,
    },
    /// The space the preview fonts take (measured on a thread).
    PreviewsSize(u64),
}
