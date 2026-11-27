//! File helpers for the downloads: clean-up of leftovers and `~/` paths.

use std::path::PathBuf;

use crate::model::paths::home;

/// Removes a folder and everything in it. A missing folder, or a file that
/// stays, does not matter: this only cleans up leftovers (a failed download,
/// a cache that is made again when needed).
pub(in crate::system) fn remove_dir_if_present(dir: &std::path::Path) {
    // Ignored on purpose: clean-up of leftovers; the caller has a better error to report.
    let _ = std::fs::remove_dir_all(dir);
}

/// Removes a file that may not exist (see [`remove_dir_if_present`]).
pub(in crate::system) fn remove_file_if_present(file: &std::path::Path) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_replaces_the_home_marker() {
        assert_eq!(expand("~/x"), home().join("x"));
        assert_eq!(expand("/x"), PathBuf::from("/x"));
    }
}
