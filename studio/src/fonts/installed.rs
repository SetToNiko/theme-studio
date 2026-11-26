//! The installed fonts: families as fontconfig (`fc-list`) reports them.

use std::collections::{BTreeMap, HashSet};
use std::process::Command;

use super::types::Installed;

/// Name parts of fonts made for other scripts or for symbols.
const SPECIAL: [&str; 5] = ["CJK", "Emoji", "Math", "Symbol", "Unifont"];

/// Installed font families, sorted by name.
pub fn installed_fonts() -> Vec<Installed> {
    let run = |args: &[&str]| {
        Command::new("fc-list")
            .args(args)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    };
    // family → (name, monospaced, has a scalable file)
    let mut fams: BTreeMap<String, (String, bool, bool)> = BTreeMap::new();
    for line in run(&["--format", "%{family[0]}\t%{spacing}\t%{fontformat}\n"]).lines() {
        let mut parts = line.split('\t');
        let family = parts.next().unwrap_or("").trim();
        let spacing = parts.next().unwrap_or("").trim();
        let format = parts.next().unwrap_or("").trim();
        if family.is_empty() {
            continue;
        }
        let e = fams
            .entry(family.to_lowercase())
            .or_insert((family.to_string(), false, false));
        e.1 |= !spacing.is_empty();
        e.2 |= !matches!(format, "PCF" | "BDF");
    }
    let nerd: HashSet<String> = run(&[":charset=f115", "--format", "%{family[0]}\n"])
        .lines()
        .map(|l| l.trim().to_lowercase())
        .collect();
    fams.into_iter()
        .map(|(lower, (family, mono, scalable))| Installed {
            nerd: nerd.contains(&lower),
            special: SPECIAL.iter().any(|s| family.contains(s)),
            bitmap: !scalable,
            family,
            mono,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_fonts_know_mono_and_nerd() {
        let fonts = installed_fonts();
        if fonts.is_empty() {
            eprintln!("skipped: no fontconfig");
            return;
        }
        let find = |n: &str| fonts.iter().find(|f| f.family == n);
        if let Some(jb) = find("JetBrainsMono Nerd Font") {
            assert!(jb.mono && jb.nerd);
        }
        if let Some(dv) = find("DejaVu Sans") {
            assert!(!dv.mono && !dv.nerd);
        }
    }
}
