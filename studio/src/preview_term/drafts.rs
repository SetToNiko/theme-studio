//! The draft colours file: its text, and the clean-up of old draft folders.

use std::path::Path;

use crate::model::palette::{KEYS, Palette};

/// The text of the draft colours file (same format as `~/.config/colors/colors.sh`).
///
/// It has one `key=#hex` line for each palette key.
pub(super) fn draft_text(p: &Palette) -> String {
    KEYS.iter()
        .map(|k| [k, "=", p.get(k).hex().as_str(), "\n"].concat())
        .collect()
}

/// Removes draft folders of studios that are no longer running (a closed
/// window kills the studio before it can clean up).
pub(super) fn remove_stale_drafts(base: &Path) {
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
    fn draft_text_has_one_line_for_each_key() {
        let p = Palette::blank();
        let text = draft_text(&p);
        assert_eq!(text.lines().count(), KEYS.len());
        assert_eq!(
            text.lines().next(),
            Some(format!("{}={}", KEYS[0], p.get(KEYS[0]).hex()).as_str())
        );
    }
}
