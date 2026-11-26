//! The curl calls of the font download: small JSON answers, big files with
//! progress, and a week-long cache for the font lists.

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::low;
use crate::model::paths::download_cache_dir;

/// GETs a URL's body (small JSON answers).
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
            let _ = std::fs::create_dir_all(download_cache_dir());
            let _ = std::fs::write(&path, v.to_string());
            Ok(v)
        }
        Err(e) => read().ok_or(e),
    }
}
