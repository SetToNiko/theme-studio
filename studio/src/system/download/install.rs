//! The install job: downloads a font's files with curl, unpacks Nerd Font
//! archives with tar, records the Nix hashes, and refreshes fontconfig's
//! cache with fc-cache. Also removes installed fonts.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use super::net::{fetch, get};
use super::{Entry, Msg, Source, low, notify, remove_dir_if_present, remove_file_if_present};

/// The google/fonts folders to look in: the license's first.
pub fn google_dirs(e: &Entry) -> Vec<String> {
    let dir = e.id.replace('-', "");
    let first = match e.license.as_str() {
        l if l.starts_with("Apache") => "apache",
        l if l.starts_with("UFL") => "ufl",
        _ => "ofl",
    };
    let mut v = vec![format!("{first}/{dir}")];
    v.extend(
        ["ofl", "apache", "ufl"]
            .iter()
            .filter(|l| **l != first)
            .map(|l| format!("{l}/{dir}")),
    );
    v
}

/// sha256 as a Nix SRI hash (for fetchurl in packaging/nix/fonts.nix).
fn sri(file: &Path) -> Result<String> {
    let out = low("sha256sum")
        .arg(file)
        .output()
        .context("running sha256sum")?;
    let hex = String::from_utf8_lossy(&out.stdout);
    let hex = hex
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("no checksum"))?;
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok())
        .collect();
    Ok(format!("sha256-{}", crate::model::encoding::base64(&bytes)))
}

/// Downloads `files` (name, url, size) into `dir`, reporting progress.
fn fetch_all(
    id: &str,
    files: &[(String, String, u64)],
    dir: &Path,
    tx: &Sender<Msg>,
) -> Result<()> {
    let total: u64 = files.iter().map(|f| f.2).sum();
    let mut before = 0;
    for (name, url, size) in files {
        fetch(url, &dir.join(name), |n| {
            notify(
                tx,
                Msg::Progress {
                    id: id.to_string(),
                    done: before + n,
                    total,
                },
            );
        })?;
        before += size;
    }
    Ok(())
}

/// The .ttf files of a GitHub folder listing: name, download URL, size.
fn ttf_files(list: &Value) -> Vec<(String, String, u64)> {
    let mut files = Vec::new();
    for f in list.as_array().into_iter().flatten() {
        let Some(name) = f.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !is_ttf(name) {
            continue;
        }
        let Some(url) = f.get("download_url").and_then(Value::as_str) else {
            continue;
        };
        let size = f.get("size").and_then(Value::as_u64).unwrap_or(0);
        files.push((name.to_string(), url.to_string(), size));
    }
    files
}

/// Whether a file name ends in `.ttf`, exactly in lower case (as the font
/// repositories name their files).
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the repositories use lower-case .ttf names; an exact match keeps what is installed the same"
)]
fn is_ttf(name: &str) -> bool {
    name.ends_with(".ttf")
}

/// Google: every .ttf of the font's folder in the google/fonts repo.
fn install_google(e: &Entry, dir: &Path, tx: &Sender<Msg>) -> Result<Value> {
    for repo_dir in google_dirs(e) {
        let Ok(text) = get(&format!(
            "https://api.github.com/repos/google/fonts/contents/{repo_dir}"
        )) else {
            continue;
        };
        let list: Value = serde_json::from_str(&text)?;
        let files = ttf_files(&list);
        if files.is_empty() {
            continue;
        }
        fetch_all(&e.id, &files, dir, tx)?;
        return Ok(json!({"source": "google", "id": e.id, "dir": repo_dir}));
    }
    // not in the repo: Fontsource's files
    install_fontsource(e, dir, tx)
}

/// Fontsource: a ttf per weight and style, latin and latin-ext.
fn install_fontsource(e: &Entry, dir: &Path, tx: &Sender<Msg>) -> Result<Value> {
    let v: Value = serde_json::from_str(&get(&format!(
        "https://api.fontsource.org/v1/fonts/{}",
        e.id
    ))?)?;
    let files = fontsource_files(e, &v);
    if files.is_empty() {
        bail!("Fontsource has no files for {}", e.family);
    }
    fetch_all(&e.id, &files, dir, tx)?;
    let mut list = Vec::new();
    for (name, url, _) in &files {
        list.push(json!({"name": name, "url": url, "hash": sri(&dir.join(name))?}));
    }
    Ok(json!({"source": "fontsource", "id": e.id, "files": list}))
}

/// The files of a Fontsource answer: name, download URL, size (unknown: 0).
/// One file per weight and style, in the latin and latin-ext subsets (or
/// the first subset when it has neither).
fn fontsource_files(e: &Entry, v: &Value) -> Vec<(String, String, u64)> {
    let mut files = Vec::new();
    for (weight, styles) in v
        .get("variants")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        for (style, subsets) in styles.as_object().into_iter().flatten() {
            let subsets = subsets.as_object().cloned().unwrap_or_default();
            let wanted: Vec<&String> = subsets
                .keys()
                .filter(|s| *s == "latin" || *s == "latin-ext")
                .collect();
            let pick = if wanted.is_empty() {
                subsets.keys().take(1).collect()
            } else {
                wanted
            };
            for subset in pick {
                if let Some(url) = subsets[subset].pointer("/url/ttf").and_then(Value::as_str) {
                    files.push((
                        format!("{}-{subset}-{weight}-{style}.ttf", e.id),
                        url.to_string(),
                        0,
                    ));
                }
            }
        }
    }
    files
}

/// Nerd Fonts: the release's `<asset>.tar.xz`, unpacked.
pub(super) fn install_nerd(e: &Entry, dir: &Path, tx: &Sender<Msg>) -> Result<Value> {
    let Source::Nerd { asset, url, size } = &e.source else {
        bail!("not a Nerd Font")
    };
    let archive = dir.join(format!("{asset}.tar.xz"));
    fetch_all(
        &e.id,
        &[(format!("{asset}.tar.xz"), url.clone(), *size)],
        dir,
        tx,
    )?;
    let out = low("tar")
        .arg("-xJf")
        .arg(&archive)
        .arg("-C")
        .arg(dir)
        .output()?;
    remove_file_if_present(&archive);
    if !out.status.success() {
        bail!(
            "unpacking {asset}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(json!({"source": "nerd", "id": e.id, "asset": asset}))
}

/// Installs a font in the background into `root/<id>/`, then refreshes
/// fontconfig's cache.
pub fn install(e: Entry, root: PathBuf, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let dir = root.join(&e.id);
        let mut warning = None;
        let result = (|| -> Result<(String, Value)> {
            std::fs::create_dir_all(&dir)?;
            let entry = match e.source {
                Source::Google => install_google(&e, &dir, &tx)?,
                Source::Fontsource => install_fontsource(&e, &dir, &tx)?,
                Source::Nerd { .. } => install_nerd(&e, &dir, &tx)?,
            };
            warning = refresh_warning("fc-cache", &root);
            let name = match &e.source {
                Source::Nerd { asset, .. } => asset.clone(),
                _ => e.family.clone(),
            };
            Ok((name, entry))
        })();
        if result.is_err() {
            remove_dir_if_present(&dir);
        }
        notify(
            &tx,
            Msg::Installed {
                id: e.id.clone(),
                result: result.map_err(|e| format!("{e:#}")),
                warning,
            },
        );
    });
}

/// Removes a downloaded font's files and refreshes fontconfig's cache.
///
/// # Errors
/// Fails when the files cannot be removed. A failed cache refresh is not an
/// error (fontconfig rescans changed folders by itself): the text of the
/// warning is returned in `Ok(Some(..))`.
pub fn remove_files(root: &Path, id: &str) -> Result<Option<String>> {
    let dir = root.join(id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
        return Ok(refresh_warning("fc-cache", root));
    }
    Ok(None)
}

/// The details when the cache refresh fails (for example
/// `fc-cache failed for <folder>: <its error>`), else `None`. The caller words
/// the message. The files are fine in that case: fontconfig rescans a changed
/// font folder by itself, and fc-cache can fail because of an unrelated broken
/// font.
fn refresh_warning(program: &str, root: &Path) -> Option<String> {
    refresh_font_cache(program, root)
        .err()
        .map(|e| format!("{e:#}"))
}

/// Runs `program` (fc-cache) on `root` so that fontconfig sees the changed fonts.
fn refresh_font_cache(program: &str, root: &Path) -> Result<()> {
    let out = low(program)
        .arg(root)
        .output()
        .with_context(|| format!("running {program}: is it installed?"))?;
    if !out.status.success() {
        bail!(
            "{program} failed for {}: {}",
            root.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::catalogue::{parse_fontsource, parse_nerd};
    use super::super::channel_pair;
    use super::super::net::cached;
    use super::*;
    use std::time::Duration;

    #[test]
    fn google_dirs_start_with_the_license_folder() {
        let fs = json!([
            {"id": "roboto-mono", "license": "Apache-2.0", "type": "google"},
            {"id": "zen-loop", "license": "OFL-1.1", "type": "other"}
        ]);
        let e = parse_fontsource(&fs);
        assert_eq!(
            google_dirs(&e[0]),
            ["apache/robotomono", "ofl/robotomono", "ufl/robotomono"]
        );
        assert_eq!(google_dirs(&e[1])[0], "ofl/zenloop");
    }

    #[test]
    fn a_failed_font_cache_refresh_is_reported() {
        let root = std::env::temp_dir();
        let err = refresh_font_cache("false", &root).unwrap_err();
        assert!(format!("{err:#}").contains("false failed"), "{err:#}");
        let err = refresh_font_cache("theme-studio-no-such-program", &root).unwrap_err();
        assert!(
            format!("{err:#}").contains("theme-studio-no-such-program failed"),
            "{err:#}"
        );
        refresh_font_cache("true", &root).unwrap();
    }

    #[test]
    fn a_failed_cache_refresh_is_a_warning_and_the_files_stay() {
        let root = std::env::temp_dir().join(format!("theme-studio-warn-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let warning = refresh_warning("false", &root).unwrap();
        assert!(warning.starts_with("false failed for "), "{warning}");
        assert!(root.exists());
        assert_eq!(refresh_warning("true", &root), None);
        // a remove with a missing folder is fine and has no warning
        assert_eq!(remove_files(&root, "no-such-font").unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sri_hashes_in_the_form_nix_expects() {
        let d = std::env::temp_dir().join(format!("theme-studio-sri-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        // the SRI form Nix expects (sha256 of "abc")
        std::fs::write(d.join("f"), "abc").unwrap();
        assert_eq!(
            sri(&d.join("f")).unwrap(),
            "sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0="
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    /// Real downloads (network): `cargo test -- --ignored live_installs`.
    #[test]
    #[ignore = "downloads from the network"]
    // Long function (reason verified, phase 6): an ignored test that installs from the network in order (download, check, remove); each step needs the files of the step before.
    fn live_installs() {
        let root = std::env::temp_dir().join(format!("theme-studio-live-{}", std::process::id()));
        remove_dir_if_present(&root);
        let (tx, rx) = channel_pair();
        let fs = cached("fontsource.json", "https://api.fontsource.org/v1/fonts").unwrap();
        let all = parse_fontsource(&fs);
        let pick = |id: &str| all.iter().find(|e| e.id == id).unwrap().clone();
        let (_, nerd) = parse_nerd(
            &cached(
                "nerd-fonts.json",
                "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest",
            )
            .unwrap(),
        );
        let heavy = nerd
            .into_iter()
            .find(|e| e.family == "HeavyData Nerd Font")
            .unwrap();
        for e in [pick("zen-loop"), pick("bagnard"), heavy] {
            let id = e.id.clone();
            install(e, root.clone(), tx.clone());
            let mut progress = 0;
            let result = loop {
                match rx.recv_timeout(Duration::from_secs(120)).unwrap() {
                    Msg::Progress { .. } => progress += 1,
                    Msg::Installed { result, .. } => break result,
                    _ => {}
                }
            };
            let (name, entry) = result.unwrap();
            let ttf = std::fs::read_dir(root.join(&id))
                .unwrap()
                .flatten()
                .filter(|f| {
                    f.path()
                        .extension()
                        .is_some_and(|x| x == "ttf" || x == "otf")
                })
                .count();
            println!("{name}: {entry}  ({ttf} font files, {progress} progress reports)");
            assert!(ttf > 0, "{name} has font files");
        }
        remove_files(&root, "zen-loop").unwrap();
        assert!(!root.join("zen-loop").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
