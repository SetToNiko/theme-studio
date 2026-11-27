//! The command line of the studio: argument handling, `--init-config`, and the start.
//!
//! `main.rs` calls [`crate::cli::run`]. It does the job that the arguments name, or starts the terminal UI.

use std::io::stdout;

use anyhow::Result;
use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;

use crate::app::{self, App, Start, Term};
use crate::fonts::presets::FontStore;
use crate::model::store::Store;
use crate::model::theme::{Theme, ThemeStore};
use crate::{config, model, system, ui};

const USAGE: &str = "\
Usage: theme-studio [--fonts] [THEME | PALETTE | --new]
       theme-studio --init-config

Edit a theme from themes.json (default: the active one), or a palette from
palettes.json, or start a new theme; --fonts starts with the keyboard on the
font roles. Press ? inside for keys.
--init-config writes any missing settings files, updates palettes.json and
fonts.json to their current file formats, makes the first theme from the
active palette and fonts if there is none, and exits.

Settings: $XDG_CONFIG_HOME/theme-studio (default ~/.config/theme-studio):
  keymaps.json   keymap presets and the active one
  settings.json  startup defaults

Environment:
  THEME_STUDIO_DATA     your palettes.json, fonts.json and themes.json
                        (default $XDG_CONFIG_HOME/theme-studio)
  THEME_STUDIO_SHARE    the files the studio and scripts need (set by the
                        package)
  THEME_STUDIO_TIMING   set: log font preview render times to
                        ~/.local/state/theme-studio/timing.log

More: docs/theme-studio.md (also the Settings page's README tab)";

/// Starts the studio, or does the job that the command line names (`--help`, `--init-config`).
///
/// # Errors
/// Fails when the palette or theme that the command line names cannot be loaded, the terminal
/// cannot be set up, or the main loop fails.
///
/// # Examples
///
/// The example is only compiled. A run would read the arguments of the test program and start the
/// terminal.
///
/// ```no_run
/// theme_studio::cli::run()?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn run() -> Result<()> {
    // every write the studio makes keeps the old file in the history
    model::history::init(model::paths::history_dir());
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let fonts_tab = args.iter().any(|a| a == "--fonts");
    args.retain(|a| a != "--fonts");
    let start = match args.first().map(String::as_str) {
        Some("-h" | "--help") => {
            println!("{USAGE}");
            return Ok(());
        }
        Some("--new") => Start::New,
        Some("--init-config") => {
            init_config();
            return Ok(());
        }
        Some(name) => Start::Named(name.to_string()),
        None => Start::Active,
    };

    // preview fonts moved from ~/.cache to ~/.local/share
    system::download::preview::migrate_previews();
    // Load before taking over the terminal, so errors print normally.
    let mut app = App::new(Store::locate(), start)?;
    if fonts_tab {
        app.open_fonts_tab();
    }
    run_in_terminal(app)
}

/// Does the job of `--init-config`: writes the missing settings files and updates the data files.
///
/// It also makes the first theme when `themes.json` does not exist. It prints the settings folder.
/// A problem with one file prints a warning and does not stop the other files.
fn init_config() {
    let dir = model::paths::studio_config_dir();
    let loaded = config::load(&dir, true);
    // palettes.json / fonts.json to their current file formats too
    let store = Store::locate();
    let fonts = FontStore {
        dir: store.dir.clone(),
        share: store.share.clone(),
    };
    let themes = ThemeStore {
        dir: store.dir.clone(),
    };
    if !themes.exists() {
        make_first_theme(&themes, &fonts);
    }

    for (what, r) in [
        ("palettes.json", store.migrate()),
        // before fonts.json: it takes the sizes from the presets
        ("themes.json", themes.migrate(&fonts)),
        ("fonts.json", fonts.migrate()),
    ] {
        match r {
            Ok(Some(v)) => println!("{what}: updated to version {v}"),
            Ok(None) => {}
            Err(e) => eprintln!("warning: {what}: {e:#}"),
        }
    }

    println!("{}", dir.display());
    for w in loaded.warnings {
        eprintln!("warning: {w}");
    }
}

/// Makes the first theme in `themes.json`, from the active palette and the active font preset.
///
/// It does nothing when there is no active palette or no active font preset. When the save fails,
/// it prints a warning.
fn make_first_theme(themes: &ThemeStore, fonts: &FontStore) {
    let (Some(p), Some(f)) = (Store::active_name(), FontStore::active_name()) else {
        return;
    };
    let theme = Theme {
        palette: p.clone(),
        sizes: fonts.load(&f).ok().map(|preset| preset.sizes()),
        fonts: f,
    };
    match themes.save(&p, &theme) {
        Ok(()) => {
            let _ = ThemeStore::set_active(&p);
            println!("themes.json: made the theme \"{p}\"");
        }
        Err(e) => eprintln!("warning: themes.json: {e:#}"),
    }
}

/// Runs the main loop of `app` in the terminal, and then gives the terminal back to the shell.
///
/// # Errors
/// Fails when the terminal cannot be set up, or the main loop fails. When the setup fails, the
/// terminal is not restored.
fn run_in_terminal(mut app: App) -> Result<()> {
    let (mut terminal, enhanced) = setup_terminal()?;
    // Which image protocol the terminal speaks (kitty graphics in Ghostty);
    // asked now, before key events are read. Half blocks if it won't say.
    app.picker = Some(
        ratatui_image::picker::Picker::from_query_stdio()
            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks()),
    );
    let result = app.run(&mut terminal, ui::draw);
    restore_terminal(enhanced);
    result
}

/// Takes over the terminal: raw mode, the alternate screen, the mouse, and the paste events.
///
/// The second value is `true` when the terminal speaks the kitty keyboard protocol.
/// [`restore_terminal`] needs it.
///
/// # Errors
/// Fails when the terminal cannot be made, or the mouse and paste events cannot be turned on.
fn setup_terminal() -> Result<(Term, bool)> {
    // raw mode, the alternate screen and the panic hook; drawing goes
    // through system::out::Out (for the hover underline styles)
    drop(ratatui::init());
    let terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
        system::out::Out::default(),
    ))?;
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    // The kitty keyboard protocol (Ghostty, kitty, foot, WezTerm…), so keys
    // like Ctrl+Shift+Z, Ctrl+1 or Ctrl+I (not Tab) arrive as themselves.
    let enhanced = ratatui::crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        let _ = execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
    }
    Ok((terminal, enhanced))
}

/// Gives the terminal back to the shell: the mouse pointer, the key protocol, the events, and the
/// screen.
///
/// `enhanced` is the second value of [`setup_terminal`].
fn restore_terminal(enhanced: bool) {
    app::set_pointer("default");
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
}
