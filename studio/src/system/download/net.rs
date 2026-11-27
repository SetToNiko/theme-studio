//! The curl calls of the font download: small JSON answers, big files with
//! progress, and a week-long cache for the font lists.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::model::paths::download_cache_dir;

/// A command at a lower priority (nice), so downloads, unpacking and font
/// scans in the background never slow the studio, nvim or the shell.
pub(in crate::system) fn low(program: &str) -> Command {
    let mut c = Command::new("nice");
    c.args(["-n", "10", program]);
    c
}

/// GETs a URL's body (small JSON answers).
///
/// # Errors
/// Fails when curl cannot start, or exits with an error (for example an HTTP error or no network).
pub(super) fn get(url: &str) -> Result<String> {
    let out = low("curl")
        .args([
            "-sS",
            "-f",
            "-L",
            "--max-time",
            "60",
            "-H",
            "Accept: application/json",
            url,
        ])
        .output()
        .context("running curl")?;
    if !out.status.success() {
        bail!("{url}: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Downloads a URL to `out`, calling `progress(bytes so far)` as it goes.
///
/// # Errors
/// Fails when curl cannot start or cannot be waited for, or exits with an error.
pub(super) fn fetch(url: &str, out: &Path, mut progress: impl FnMut(u64)) -> Result<()> {
    let mut child = low("curl")
        .args(["-sS", "-f", "-L", "--max-time", "600", "-o"])
        .arg(out)
        .arg(url)
        .stderr(Stdio::piped())
        .spawn()
        .context("running curl")?;
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                let mut err = String::new();
                if let Some(mut e) = child.stderr.take() {
                    use std::io::Read;
                    // Ignored on purpose: this only adds detail to the error below.
                    let _ = e.read_to_string(&mut err);
                }
                bail!("{url}: {}", err.trim());
            }
            progress(std::fs::metadata(out).map_or(0, |m| m.len()));
            return Ok(());
        }
        progress(std::fs::metadata(out).map_or(0, |m| m.len()));
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// A JSON answer, cached for a week (an old copy is used when offline).
///
/// # Errors
/// Fails when the cache is too old (or not usable) and the download fails or is not valid JSON, and
/// no old copy can be read.
pub(super) fn cached(name: &str, url: &str) -> Result<Value> {
    let path = download_cache_dir().join(name);
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < Duration::from_hours(7 * 24));
    let read =
        || -> Option<Value> { serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok() };
    if fresh && let Some(v) = read() {
        return Ok(v);
    }
    match get(url).and_then(|t| Ok(serde_json::from_str::<Value>(&t)?)) {
        Ok(v) => {
            // Ignored on purpose: the cache only saves a later download; the answer `v` is good.
            let _ = std::fs::create_dir_all(download_cache_dir());
            let _ = std::fs::write(&path, v.to_string());
            Ok(v)
        }
        Err(e) => read().ok_or(e),
    }
}
