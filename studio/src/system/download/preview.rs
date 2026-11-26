//! The preview font cache: one small font file per previewed font, fetched
//! in the background, plus the fontconfig setup that lets a program draw
//! with it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use anyhow::{Context, Result, anyhow};

use super::install::install_nerd;
use super::net::fetch;
use super::{Entry, Msg, Source, low};
use crate::model::paths::{
    SYSTEM_FONTCONFIG, download_cache_dir, installed_fonts_dir, preview_cache_dir,
};

/// Moves previews from where older versions kept them
/// (`~/.cache/theme-studio/preview/`), keeping only each font's Regular.
pub fn migrate_previews() {
    let (old, new) = (download_cache_dir().join("preview"), preview_cache_dir());
    if !old.is_dir() || new.exists() {
        return;
    }
    if let Some(parent) = new.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // on another file system the old ones stay (and are fetched again)
    if std::fs::rename(&old, &new).is_ok()
        && let Ok(dirs) = std::fs::read_dir(&new)
    {
        for d in dirs.flatten() {
            keep_regular_only(&d.path());
        }
    }
}

/// A fontconfig setup for one previewed font: the system's fonts plus that
/// font's folder. Much quicker to load than one listing every preview
/// (fontconfig checks each folder it's given on every start).
pub fn preview_fontconfig_for(id: &str) -> PathBuf {
    let dir = download_cache_dir().join("fontconfig-confs");
    let path = dir.join(format!("{id}.conf"));
    let conf = format!(
        "<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n<fontconfig>\n  \
         <include ignore_missing=\"yes\">{}</include>\n  <dir>{}</dir>\n  \
         <cachedir>{}</cachedir>\n</fontconfig>\n",
        SYSTEM_FONTCONFIG,
        preview_cache_dir().join(id).display(),
        download_cache_dir().join("fontconfig").display()
    );
    if std::fs::read_to_string(&path).ok().as_deref() != Some(conf.as_str()) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&path, conf);
    }
    path
}

/// Whether a font's preview file is already here.
pub fn has_preview(id: &str) -> bool {
    preview_file(&preview_cache_dir().join(id)).is_some()
}

/// The space the preview fonts take, in bytes.
pub fn previews_size() -> u64 {
    fn walk(p: &Path) -> u64 {
        std::fs::read_dir(p).map_or(0, |entries| {
            entries
                .flatten()
                .map(|e| match e.file_type() {
                    Ok(t) if t.is_dir() => walk(&e.path()),
                    _ => e.metadata().map_or(0, |m| m.len()),
                })
                .sum()
        })
    }
    walk(&preview_cache_dir())
}

/// Deletes every preview font (they're fetched again when needed).
pub fn clear_previews() -> Result<()> {
    let _ = std::fs::remove_dir_all(download_cache_dir().join("fontconfig-confs"));
    let dir = preview_cache_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(())
}

/// Leaves only the face a preview draws with (a Nerd Font archive unpacks
/// to dozens).
fn keep_regular_only(dir: &Path) {
    let Some(keep) = preview_file(dir) else {
        return;
    };
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p != keep {
                let _ = if p.is_dir() {
                    std::fs::remove_dir_all(&p)
                } else {
                    std::fs::remove_file(&p)
                };
            }
        }
    }
}

/// The family name inside a font file (fc-scan).
fn family_of(file: &Path) -> Option<String> {
    let out = low("fc-scan")
        .args(["--format", "%{family[0]}"])
        .arg(file)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

/// The font file to read the family from: a Regular, not a Mono / Propo
/// variant, when there's a choice.
fn preview_file(dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|f| f.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ttf" || x == "otf"))
        .collect();
    files.sort_by_key(|p| {
        let n = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        (
            n.contains("Mono-") || n.contains("Propo-"),
            !n.contains("Regular"),
            n.len(),
        )
    });
    files.into_iter().next()
}

/// Fetches a font for its preview in the background (Google / Fontsource:
/// one file of the regular weight; Nerd Fonts: the archive, of which only
/// the Regular is kept), unless already here.
pub fn fetch_preview(e: Entry, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let dir = preview_cache_dir().join(&e.id);
        let result = fetch_preview_family(&e, &dir, &tx);
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        let _ = tx.send(Msg::Preview {
            id: e.id.clone(),
            result: result.map_err(|e| format!("{e:#}")),
        });
    });
}

/// Gets the preview font into `dir` (unless it is there) and returns its
/// family name.
fn fetch_preview_family(e: &Entry, dir: &Path, tx: &Sender<Msg>) -> Result<String> {
    if let Some(f) = preview_file(dir) {
        return family_of(&f).ok_or_else(|| anyhow!("no family name in {}", f.display()));
    }
    std::fs::create_dir_all(dir)?;
    if let Source::Nerd { .. } = &e.source {
        install_nerd(e, dir, tx)?;
        keep_regular_only(dir);
    } else {
        // the regular weight, else the nearest
        let w = e
            .weights
            .iter()
            .min_by_key(|w| w.abs_diff(400))
            .copied()
            .unwrap_or(400);
        let url = format!(
            "https://cdn.jsdelivr.net/fontsource/fonts/{}@latest/{}-{w}-normal.ttf",
            e.id, e.subset
        );
        fetch(&url, &dir.join(format!("{}-{w}.ttf", e.id)), |_| {})?;
    }
    let f = preview_file(dir).ok_or_else(|| anyhow!("no font file came with it"))?;
    family_of(&f).ok_or_else(|| anyhow!("no family name in {}", f.display()))
}

/// An installed font's family name, read from its files in the background
/// (the Installed tab's preview: nothing to fetch).
pub fn scan_installed(id: String, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let dir = installed_fonts_dir().join(&id);
        let result = preview_file(&dir)
            .ok_or_else(|| format!("no font files in {}", dir.display()))
            .and_then(|f| {
                family_of(&f).ok_or_else(|| format!("no family name in {}", f.display()))
            });
        let _ = tx.send(Msg::Preview { id, result });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_keep_only_the_regular() {
        let dir = std::env::temp_dir().join(format!("theme-studio-keep-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in [
            "HackNerdFont-Regular.ttf",
            "HackNerdFontMono-Regular.ttf",
            "HackNerdFont-Bold.ttf",
            "LICENSE.md",
            "sub/x.ttf",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        keep_regular_only(&dir);
        let left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into())
            .collect();
        assert_eq!(left, ["HackNerdFont-Regular.ttf"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
