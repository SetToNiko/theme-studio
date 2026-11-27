//! Characterization tests for the command line of the `theme-studio` binary.
//!
//! Each test runs the real binary with `std::process::Command`. It tests only
//! the arguments that end before the terminal starts: `-h`, `--help`,
//! `--init-config`, and a name that the studio cannot load.
//! The tests record the CURRENT behaviour, also where it looks incorrect.
//!
//! Safety: each run gets its own temporary folder. The folder holds `HOME`, the
//! `XDG_*` folders, and the `THEME_STUDIO_*` folders, so a run never touches the
//! real files of the user. The tests never change the environment of the test
//! process. A run that does not end in 10 seconds is killed and fails the test.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The longest time that one run of the binary can take.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The text that `--help` and `-h` print: the first line of the usage.
const USAGE_FIRST_LINE: &str = "Usage: theme-studio [--fonts] [THEME | PALETTE | --new]";

/// Counts the folders that this test process made, so that each name is unique.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// What one run of the binary gave back.
struct Run {
    /// The exit code of the process.
    code: i32,
    /// Everything that the process wrote to standard output.
    stdout: String,
    /// Everything that the process wrote to standard error.
    stderr: String,
}

/// A temporary folder tree for one test. Drop removes it.
struct Sandbox {
    /// The root of the tree.
    root: PathBuf,
}

impl Sandbox {
    /// Makes a new empty tree with a unique name in the system temporary folder.
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "theme-studio-command-line-{}-{n}",
            std::process::id()
        ));
        for sub in [
            "home", "config", "data", "state", "cache", "run", "store", "share",
        ] {
            fs::create_dir_all(root.join(sub)).expect("make a sandbox folder");
        }
        Self { root }
    }

    /// A folder of the tree.
    fn dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// The settings folder of the studio (`$XDG_CONFIG_HOME/theme-studio`).
    fn settings_dir(&self) -> PathBuf {
        self.dir("config").join("theme-studio")
    }

    /// Copies the palettes and fonts files of `tests/fixtures/data` to the store folder.
    fn copy_fixtures(&self) {
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/data");
        for name in ["palettes.json", "fonts.json"] {
            fs::copy(from.join(name), self.dir("store").join(name)).expect("copy a fixture");
        }
    }

    /// Writes the state files of the switch scripts: the active palette and fonts.
    fn set_active(&self, palette: &str, fonts: &str) {
        for (folder, value) in [("colors", palette), ("fonts", fonts)] {
            let dir = self.dir("home").join(".config").join(folder);
            fs::create_dir_all(&dir).expect("make a state folder");
            fs::write(dir.join("current"), format!("{value}\n")).expect("write a state file");
        }
    }

    /// Runs the binary with `args` and waits at most [`TIMEOUT`].
    fn run(&self, args: &[&str]) -> Run {
        let mut child = Command::new(env!("CARGO_BIN_EXE_theme-studio"))
            .args(args)
            .env("HOME", self.dir("home"))
            .env("XDG_CONFIG_HOME", self.dir("config"))
            .env("XDG_DATA_HOME", self.dir("data"))
            .env("XDG_STATE_HOME", self.dir("state"))
            .env("XDG_CACHE_HOME", self.dir("cache"))
            .env("XDG_RUNTIME_DIR", self.dir("run"))
            .env("THEME_STUDIO_DATA", self.dir("store"))
            .env("THEME_STUDIO_SHARE", self.dir("share"))
            .env_remove("THEME_STUDIO_TIMING")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the binary");
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().expect("wait for the binary") {
                break status;
            }
            if start.elapsed() > TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the binary did not end in {TIMEOUT:?}: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut run = Run {
            code: status.code().expect("the binary ended by a signal"),
            stdout: String::new(),
            stderr: String::new(),
        };
        child
            .stdout
            .take()
            .expect("stdout pipe")
            .read_to_string(&mut run.stdout)
            .expect("read stdout");
        child
            .stderr
            .take()
            .expect("stderr pipe")
            .read_to_string(&mut run.stderr)
            .expect("read stderr");
        run
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Reads a file of the tree as text.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn short_help_flag_prints_usage_and_exits_zero() {
    let sb = Sandbox::new();
    let run = sb.run(&["-h"]);
    assert_eq!(run.code, 0);
    assert!(run.stdout.starts_with(USAGE_FIRST_LINE), "{}", run.stdout);
    assert!(run.stdout.contains("--init-config"));
    assert!(run.stdout.contains("THEME_STUDIO_DATA"));
    assert_eq!(run.stderr, "");
}

#[test]
fn long_help_flag_prints_the_same_usage_as_the_short_flag() {
    let sb = Sandbox::new();
    let short = sb.run(&["-h"]);
    let long = sb.run(&["--help"]);
    assert_eq!(long.code, 0);
    assert_eq!(long.stdout, short.stdout);
    assert!(
        long.stdout
            .ends_with("(also the Settings page's README tab)\n")
    );
}

#[test]
fn help_writes_no_settings_files() {
    let sb = Sandbox::new();
    sb.run(&["--help"]);
    assert!(!sb.settings_dir().exists());
    assert!(!sb.dir("store").join("themes.json").exists());
}

#[test]
fn help_after_the_fonts_flag_still_prints_usage() {
    // `--fonts` is removed from the arguments first, so it can come before `--help`.
    let sb = Sandbox::new();
    let run = sb.run(&["--fonts", "--help"]);
    assert_eq!(run.code, 0);
    assert!(run.stdout.starts_with(USAGE_FIRST_LINE));
}

#[test]
fn help_is_ignored_when_it_is_not_the_first_argument() {
    // INCORRECT BEHAVIOUR (studio/src/main.rs:64): only the first argument is
    // tested for `--help`. `theme-studio nosuch --help` treats `nosuch` as a name.
    let sb = Sandbox::new();
    sb.copy_fixtures();
    let run = sb.run(&["nosuch", "--help"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.contains("no palette named 'nosuch'"),
        "{}",
        run.stderr
    );
}

#[test]
fn init_config_without_data_files_writes_settings_and_exits_zero() {
    let sb = Sandbox::new();
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, format!("{}\n", sb.settings_dir().display()));
    assert_eq!(run.stderr, "");
    for name in ["keymaps.json", "settings.json"] {
        let text = read(&sb.settings_dir().join(name));
        let _: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    }
    // No palettes, fonts, or active palette: the data files are not made.
    for name in ["palettes.json", "fonts.json", "themes.json"] {
        assert!(!sb.dir("store").join(name).exists(), "{name}");
    }
}

#[test]
fn init_config_twice_keeps_the_settings_files() {
    let sb = Sandbox::new();
    sb.run(&["--init-config"]);
    let keymaps = read(&sb.settings_dir().join("keymaps.json"));
    let settings = read(&sb.settings_dir().join("settings.json"));
    let again = sb.run(&["--init-config"]);
    assert_eq!(again.code, 0);
    assert_eq!(again.stdout, format!("{}\n", sb.settings_dir().display()));
    assert_eq!(read(&sb.settings_dir().join("keymaps.json")), keymaps);
    assert_eq!(read(&sb.settings_dir().join("settings.json")), settings);
}

#[test]
fn init_config_makes_the_first_theme_from_the_active_palette_and_fonts() {
    let sb = Sandbox::new();
    sb.copy_fixtures();
    sb.set_active("0x96f", "JetBrains Mono");
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert!(
        run.stdout
            .contains("themes.json: made the theme \"0x96f\"\n"),
        "{}",
        run.stdout
    );
    let themes = read(&sb.dir("store").join("themes.json"));
    assert!(themes.contains("\"0x96f\""));
    assert!(themes.contains("\"JetBrains Mono\""));
    assert!(themes.contains("\"monospace\": 14"), "{themes}");
}

#[test]
fn init_config_twice_keeps_the_first_theme() {
    let sb = Sandbox::new();
    sb.copy_fixtures();
    sb.set_active("0x96f", "JetBrains Mono");
    sb.run(&["--init-config"]);
    let files = ["palettes.json", "fonts.json", "themes.json"];
    let before: Vec<String> = files
        .iter()
        .map(|f| read(&sb.dir("store").join(f)))
        .collect();
    let again = sb.run(&["--init-config"]);
    assert_eq!(again.code, 0);
    assert!(!again.stdout.contains("made the theme"), "{}", again.stdout);
    assert!(
        !again.stdout.contains("updated to version"),
        "{}",
        again.stdout
    );
    for (f, old) in files.iter().zip(&before) {
        assert_eq!(&read(&sb.dir("store").join(f)), old, "{f}");
    }
}

#[test]
fn init_config_makes_no_theme_without_an_active_palette() {
    let sb = Sandbox::new();
    sb.copy_fixtures();
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert!(!run.stdout.contains("made the theme"));
    assert!(!sb.dir("store").join("themes.json").exists());
}

#[test]
fn init_config_updates_a_palettes_file_without_a_version() {
    let sb = Sandbox::new();
    let path = sb.dir("store").join("palettes.json");
    fs::write(&path, "{\"a\":{\"bg0\":\"#000000\"}}").expect("write palettes.json");
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert!(
        run.stdout.contains("palettes.json: updated to version 2\n"),
        "{}",
        run.stdout
    );
    let text = read(&path);
    assert!(text.contains("\"version\": 2"), "{text}");
    assert!(text.contains("\"palettes\""), "{text}");
    // The second run finds the current version and says nothing.
    let again = sb.run(&["--init-config"]);
    assert!(!again.stdout.contains("updated to version"));
    assert_eq!(read(&path), text);
}

#[test]
fn init_config_warns_about_a_broken_keymaps_file_and_keeps_it() {
    let sb = Sandbox::new();
    fs::create_dir_all(sb.settings_dir()).expect("make the settings folder");
    let keymaps = sb.settings_dir().join("keymaps.json");
    fs::write(&keymaps, "{bad").expect("write keymaps.json");
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert!(run.stderr.contains("warning: parsing "), "{}", run.stderr);
    assert!(
        run.stderr.contains("(using the built-in Vim keys)"),
        "{}",
        run.stderr
    );
    assert_eq!(read(&keymaps), "{bad");
}

#[test]
fn init_config_warns_about_a_broken_palettes_file() {
    let sb = Sandbox::new();
    fs::write(sb.dir("store").join("palettes.json"), "not json").expect("write palettes.json");
    let run = sb.run(&["--init-config"]);
    assert_eq!(run.code, 0);
    assert!(
        run.stderr.contains("warning: palettes.json: parsing "),
        "{}",
        run.stderr
    );
}

#[test]
fn unknown_name_fails_with_an_error_and_exit_code_one() {
    let sb = Sandbox::new();
    sb.copy_fixtures();
    let run = sb.run(&["nosuch"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "Error: no palette named 'nosuch'\n");
}

#[test]
fn unknown_option_is_taken_as_a_name() {
    // INCORRECT BEHAVIOUR (studio/src/main.rs:121): an argument that starts with
    // `--` but is not known is looked up as a palette name. The error does not
    // say that the option is unknown.
    let sb = Sandbox::new();
    sb.copy_fixtures();
    let run = sb.run(&["--bogus"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr, "Error: no palette named '--bogus'\n");
}

#[test]
fn unknown_name_with_the_fonts_flag_fails_the_same_way() {
    let sb = Sandbox::new();
    sb.copy_fixtures();
    let run = sb.run(&["--fonts", "nosuch"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr, "Error: no palette named 'nosuch'\n");
}

#[test]
fn missing_palettes_file_fails_with_an_error_and_exit_code_one() {
    let sb = Sandbox::new();
    let run = sb.run(&["nosuch"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.starts_with("Error: reading "), "{}", run.stderr);
    assert!(run.stderr.contains("palettes.json"), "{}", run.stderr);
    assert!(run.stderr.contains("Caused by:"), "{}", run.stderr);
}
