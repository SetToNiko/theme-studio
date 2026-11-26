//! Downloading fonts (the F2 page). Google fonts are listed by the Fontsource
//! API and downloaded whole from the google/fonts GitHub repo; other
//! Fontsource fonts come from Fontsource (latin + latin-ext); Nerd Fonts come
//! from the latest ryanoasis/nerd-fonts release. Files go to
//! `~/.local/share/fonts/theme-studio/<id>/`, and each download is added to
//! the installed-fonts list that packaging/nix/fonts.nix reads, so a rebuild makes it
//! permanent. Network and archive work goes through curl, tar and fc-cache,
//! in background threads.
//! - `net.rs`: curl calls and the cached JSON answers.
//! - `catalogue.rs`: the font lists of Fontsource and Nerd Fonts.
//! - `install.rs`: the install job (download, unpack, fc-cache) and removal.
//! - `preview.rs`: the preview font cache.
//! - `list.rs`: the installed-fonts list file.
//! - This file: the types the other files share.

pub mod catalogue;
pub mod install;
pub mod list;
pub mod net;
pub mod preview;

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, channel};

use serde_json::Value;

use crate::model::paths::home;

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

/// A command at a lower priority (nice), so downloads, unpacking and font
/// scans in the background never slow the studio, nvim or the shell.
pub(super) fn low(program: &str) -> Command {
    let mut c = Command::new("nice");
    c.args(["-n", "10", program]);
    c
}

/// Sends a message to the page. The page may be closed already, so that the
/// receiver is gone; then nobody needs the message and the error is ignored.
pub(super) fn notify(tx: &Sender<Msg>, msg: Msg) {
    // Ignored on purpose: a closed page means nobody waits for the answer.
    let _ = tx.send(msg);
}

/// Removes a folder and everything in it. A missing folder, or a file that
/// stays, does not matter: this only cleans up leftovers (a failed download,
/// a cache that is made again when needed).
pub(super) fn remove_dir_if_present(dir: &std::path::Path) {
    // Ignored on purpose: clean-up of leftovers; the caller has a better error to report.
    let _ = std::fs::remove_dir_all(dir);
}

/// Removes a file that may not exist (see [`remove_dir_if_present`]).
pub(super) fn remove_file_if_present(file: &std::path::Path) {
    // Ignored on purpose: clean-up of a leftover file.
    let _ = std::fs::remove_file(file);
}

/// Expands a leading `~/`.
pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None => PathBuf::from(p),
    }
}

/// A channel for the page's background work.
pub fn channel_pair() -> (Sender<Msg>, Receiver<Msg>) {
    channel()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_replaces_the_home_marker() {
        assert_eq!(expand("~/x"), home().join("x"));
        assert_eq!(expand("/x"), PathBuf::from("/x"));
    }
}
