//! The only place that starts the switch scripts (`theme-switch`,
//! `font-switch`). They write the system's colour and font files and restart
//! i3 and Ghostty. Nothing else in the studio runs them.
//!
//! In tests the scripts never run: [`switch_command`] gives a harmless
//! command that fails with "... is off in tests". A test that means it sets
//! `THEME_STUDIO_REAL_SWITCH`.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::model::store::Store;

/// Makes a saved palette the system theme: runs `theme-switch <name>` and
/// waits for it.
///
/// # Errors
/// Returns an error if the script cannot start, or if it fails (the error
/// holds the script's own message).
pub fn apply_theme(store: &Store, name: &str) -> Result<()> {
    let mut cmd = switch_command("theme-switch", &store.dir, &store.share);
    cmd.arg(name);
    run(cmd, "theme-switch.sh")
}

/// Makes a saved font preset the system fonts: runs `font-switch <name>` at
/// the theme's sizes (`FONT_SIZES`; without it `font-switch` would use the
/// active theme's, which may be another theme). With `quiet`, Ghostty and i3
/// are not restarted (`NO_RESTART`), because a palette applied next restarts
/// them once for both.
///
/// # Errors
/// Returns an error if the script cannot start, or if it fails (the error
/// holds the script's own message).
pub fn apply_fonts(
    data: &Path,
    share: &Path,
    name: &str,
    sizes: [u32; 3],
    quiet: bool,
) -> Result<()> {
    let mut cmd = switch_command("font-switch", data, share);
    cmd.env("FONT_SIZES", sizes.map(|s| s.to_string()).join(" "));
    if quiet {
        cmd.env("NO_RESTART", "1");
    }
    cmd.arg(name);
    run(cmd, "font-switch.sh")
}

/// Runs a switch command without input and without output, and waits.
/// `what` names the script in the error text.
///
/// # Errors
/// Fails when the command cannot start, or exits with an error. The error holds what the command
/// wrote to stderr.
fn run(mut cmd: Command, what: &str) -> Result<()> {
    let out = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("running {what}"))?;
    if !out.status.success() {
        bail!("{what}: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// A switch script (`theme-switch`, `font-switch`, `themes-switch`) ready
/// to run: the command on PATH, else `bash <script>.sh` from the scripts
/// folder beside the share folder. It's told where the data and share
/// folders are, so it reads what the studio reads.
fn switch_command(name: &str, data: &Path, share: &Path) -> Command {
    // Tests never touch the real desktop: the switch scripts write
    // ~/.config/colors and ~/.config/fonts and restart i3, whatever data
    // folder they're given. A test that means it sets THEME_STUDIO_REAL_SWITCH.
    if cfg!(test) && std::env::var_os("THEME_STUDIO_REAL_SWITCH").is_none() {
        let mut c = Command::new("sh");
        c.args(["-c", "echo \"$0 is off in tests\" >&2; exit 1", name]);
        return c;
    }
    let mut cmd = if on_path(name) {
        Command::new(name)
    } else {
        let script = share
            .parent()
            .unwrap_or(share)
            .join("scripts")
            .join(format!("{name}.sh"));
        let mut c = Command::new("bash");
        c.arg(script);
        c
    };
    cmd.env("THEME_STUDIO_DATA", data);
    if share.exists() {
        cmd.env("THEME_STUDIO_SHARE", share);
    }
    cmd
}

/// Whether a command is on PATH.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|d| {
            let p = d.join(name);
            p.is_file()
                && std::fs::metadata(&p).is_ok_and(|m| {
                    std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o111 != 0
                })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_never_run_the_switch_scripts() {
        let d = std::env::temp_dir();
        let err = apply_fonts(&d, &d, "x", [1, 1, 1], true).unwrap_err();
        assert!(err.to_string().contains("font-switch is off in tests"));
    }
}
