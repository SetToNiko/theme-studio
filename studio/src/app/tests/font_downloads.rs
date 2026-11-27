//! Tests of font downloads and previews: the download page, installed fonts, prefetching, and the tournament.

use super::*;
use crate::keymap::context::Context;
use std::time::Duration;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one walk through the Download page (list, search, remove); each step uses the state the step before left"
)]
fn download_page_lists_searches_and_removes() {
    use crate::app::state::hit::Target as T;
    use crate::fonts::pages::download::{DlRow, DlStatus, DlTab};
    use crate::system::download::types::{Catalogue, Entry, Source};
    let store = temp_store("dl");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let entry = |id: &str, family: &str, cat: &str, source: Source| Entry {
        id: id.into(),
        family: family.into(),
        category: cat.into(),
        license: "OFL-1.1".into(),
        weights: vec![400, 700],
        italic: false,
        variable: false,
        source,
        subset: "latin".into(),
    };
    app.dl.catalogue = Some(Catalogue {
        google: vec![
            entry("inter", "Inter", "sans-serif", Source::Google),
            entry("roboto-mono", "Roboto Mono", "monospace", Source::Google),
            entry("zen-loop", "Zen Loop", "display", Source::Google),
        ],
        nerd: vec![entry(
            "nerd-heavydata",
            "HeavyData Nerd Font",
            "nerd font",
            Source::Nerd {
                asset: "HeavyData".into(),
                url: "u".into(),
                size: 1,
            },
        )],
        nerd_tag: "v3.5.1".into(),
    });
    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.page, Page::Downloads);
    assert_eq!(app.contexts()[0], Context::Downloads);
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Google Fonts")
            && screen.contains("Roboto Mono")
            && screen.contains("3 fonts"),
        "{screen}"
    );
    // a category: right steps to Sans
    press(&mut app, "l");
    assert_eq!(app.dl_rows(), vec![DlRow::Entry(0)]);
    press(&mut app, "h");
    // search: / then typing; Enter leaves the box
    press(&mut app, "/mono\n");
    assert_eq!(app.dl_rows(), vec![DlRow::Entry(1)]);
    assert!(render(&mut app, 160, 45).contains("Install"));
    // the preview: Roboto Mono shows code; t / L / + change it
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("Preview  Code ▾  Rust ▾  12pt"), "{screen}");
    press(&mut app, "L+");
    assert!(render(&mut app, 160, 45).contains("Preview  Code ▾  Python ▾  13pt"));
    // > pages through the code, < comes back
    press(&mut app, ">");
    let pv = app.dl.preview.as_ref().unwrap();
    assert!(pv.page > 0 && pv.page == pv.step, "{} {}", pv.page, pv.step);
    press(&mut app, "<");
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 0);
    // the wheel over the preview moves three lines
    render(&mut app, 160, 45);
    let r = app
        .hits
        .rect_of(crate::app::state::hit::Target::DlPreview)
        .unwrap();
    mouse(&mut app, MouseEventKind::ScrollDown, r.x + 2, r.y + 2);
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 3);
    // the language dropdown: a click opens it on the current one,
    // a click on a row chooses (and starts at the top)
    click_on(&mut app, crate::app::state::hit::Target::PreviewMenu(1));
    assert!(matches!(
        app.overlay,
        Overlay::Menu {
            menu: 1,
            sel: 1,
            ..
        }
    ));
    assert!(render(&mut app, 160, 45).contains(" Haskell "));
    click_on(&mut app, crate::app::state::hit::Target::OverlayRow(10));
    assert_eq!(app.dl.preview.as_ref().unwrap().lang, 10);
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 0);
    assert!(render(&mut app, 160, 45).contains("Code ▾  Haskell ▾"));
    // the kind dropdown: keys move, Enter chooses; a click outside closes
    click_on(&mut app, crate::app::state::hit::Target::PreviewMenu(0));
    press(&mut app, "j\n");
    assert!(matches!(app.overlay, Overlay::None));
    assert!(render(&mut app, 160, 45).contains("Preview  Icons ▾  13pt"));
    click_on(&mut app, crate::app::state::hit::Target::PreviewMenu(0));
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert!(matches!(app.overlay, Overlay::None));
    app.dl.preview.as_mut().unwrap().kind = Some(crate::model::samples::Kind::Code);
    press(&mut app, "t");
    assert!(render(&mut app, 160, 45).contains("Preview  Icons"));
    // a failed fetch says so
    app.dl
        .preview
        .as_mut()
        .unwrap()
        .families
        .insert("roboto-mono".into(), Err("offline".into()));
    assert!(render(&mut app, 160, 45).contains("Couldn't fetch it: offline"));
    app.dl.search.clear();
    // something in the list counts as installed and can be removed
    let list = app.installed_list_path();
    let mut m = serde_json::Map::new();
    m.insert(
        "Zen Loop".into(),
        serde_json::json!({"source": "google", "id": "zen-loop"}),
    );
    crate::system::download::list::write_list(&list, m).unwrap();

    let zen = app.dl_entries()[2].clone();
    let listed = crate::system::download::list::read_list(&list);
    assert_eq!(app.dl_status(&zen, &listed), DlStatus::Listed);
    // the Installed tab (a click) lists it; d removes it
    render(&mut app, 160, 45);
    click_on(&mut app, T::DlTab(2));
    assert_eq!(app.dl.tab, DlTab::Installed);
    assert_eq!(app.dl_rows(), vec![DlRow::Listed("Zen Loop".into())]);
    press(&mut app, "d");
    assert_eq!(crate::system::download::list::read_list(&list), Map::new());
    assert_eq!(app.dl_rows(), []);
    // Nerd Fonts tab, with the release (past the Tournament tab)
    press(&mut app, "]");
    assert_eq!(app.dl.tab, DlTab::Tournament);
    press(&mut app, "]");
    assert_eq!(app.dl.tab, DlTab::Google, "wraps around");
    press(&mut app, "]");
    assert!(render(&mut app, 160, 45).contains("release v3.5.1"));
    // up from the first row reaches the tabs
    press(&mut app, "k");
    assert!(app.dl.tabs_focused);
    press(&mut app, "j\x1b");
    assert_eq!(app.page, Page::Main);
    render(&mut app, 100, 40); // narrow
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn installed_fonts_preview_from_their_files() {
    // a real font file to stand in (skipped without one, e.g. in the
    // Nix build's sandbox)
    let Some(file) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "monospace"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .filter(|f| has_font_extension(f))
    else {
        eprintln!("skipped: no font file");
        return;
    };
    let store = temp_store("installed-preview");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let fonts = crate::model::paths::installed_fonts_dir().join("test-mono");
    std::fs::create_dir_all(&fonts).unwrap();
    let ext = if has_extension(&file, "otf") {
        "otf"
    } else {
        "ttf"
    };
    std::fs::copy(&file, fonts.join(format!("TestMono-Regular.{ext}"))).unwrap();
    let list = app.installed_list_path();
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(
        &list,
        r#"{"version": 1, "fonts": {"Test Mono": {"source": "fontsource", "id": "test-mono"}}}"#,
    )
    .unwrap();
    app.page = Page::Downloads;
    app.dl.tab = crate::fonts::pages::download::DlTab::Installed;
    app.dl.listed = None;
    app.dl.sel = 0;
    // the highlighted installed font is the preview's, with code by default
    let e = app.preview_entry().unwrap();
    assert_eq!(
        (e.id.as_str(), e.category.as_str()),
        ("test-mono", "monospace")
    );
    // its family comes from its files, without fetching anything
    let start = std::time::Instant::now();
    loop {
        app.poll_downloads();
        if let Some(r) = app
            .dl
            .preview
            .as_ref()
            .and_then(|p| p.families.get("test-mono"))
        {
            assert!(r.as_ref().is_ok_and(|f| !f.is_empty()), "{r:?}");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no family name");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::remove_dir_all(fonts).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn prefetching_keeps_the_main_loop_quick() {
    use crate::system::download::types::{Catalogue, Entry, Msg, Source};
    use std::sync::mpsc::Sender;
    // downloads answer at once (no network)
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the prefetch field is a function pointer that takes the entry and the sender by value"
    )]
    fn stub(e: Entry, tx: Sender<Msg>) {
        let _ = tx.send(Msg::Preview {
            id: e.id,
            result: Ok(e.family),
        });
    }
    let store = temp_store("prefetch-loop");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let entry = |i: usize, category: &str, source: Source| Entry {
        id: format!("font-{i}"),
        family: format!("Font {i}"),
        category: category.into(),
        license: String::new(),
        weights: vec![400],
        italic: false,
        variable: false,
        source,
        subset: "latin".into(),
    };
    let nerd = |i| Source::Nerd {
        asset: format!("N{i}"),
        url: String::new(),
        size: 1,
    };
    app.dl.catalogue = Some(Catalogue {
        google: (0..2000)
            .map(|i| entry(i, "monospace", Source::Google))
            .collect(),
        nerd: (2000..2005)
            .map(|i| entry(i, "nerd font", nerd(i)))
            .collect(),
        nerd_tag: String::new(),
    });
    app.settings.fonts_page.prefetch = true;
    app.settings.fonts_page.prefetch_categories = vec!["monospace".into(), "nerd font".into()];
    app.prefetch.fetch = Some(stub);
    let (start, mut slowest) = (Instant::now(), Duration::ZERO);
    while app.prefetch.done < 2005 || app.prefetch.active() {
        let t = Instant::now();
        app.poll_downloads();
        app.poll_prefetch();
        slowest = slowest.max(t.elapsed());
        assert!(
            app.prefetch.nerd_in_flight.len() <= 1,
            "archives one at a time"
        );
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "stuck at {}",
            app.prefetch.done
        );
    }
    assert_eq!((app.prefetch.done, app.prefetch.total), (2005, 2005));
    // a tick never takes long (generous for a debug build)
    assert!(
        slowest < Duration::from_millis(20),
        "a tick took {slowest:?}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_tournament_of_installed_fonts_from_setup_to_results() {
    use crate::fonts::pages::tournament::Stage;
    let store = temp_store("tournament");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let _ = std::fs::remove_dir_all(crate::model::tournament::dir());
    app.fonts.installed = (0..5)
        .map(|i| crate::fonts::types::Installed {
            family: format!("Test Mono {i}"),
            mono: true,
            nerd: false,
            bitmap: false,
            special: false,
        })
        .collect();
    app.page = Page::Downloads;
    app.dl.tab = crate::fonts::pages::download::DlTab::Tournament;
    let screen = |app: &mut App| render(app, 160, 45);
    assert!(screen(&mut app).contains("Installed monospaced"));
    // the installed monospaced pool: what it takes, then it starts
    app.tour().sel = 6;
    press(&mut app, "\n");
    let Overlay::Ask { question, .. } = &app.overlay else {
        panic!("asks first")
    };
    assert!(
        question.contains("5 fonts, 4 picks; nothing to fetch"),
        "{question}"
    );
    press(&mut app, "y");
    assert_eq!(app.tour().stage, Stage::Fetching);
    app.poll_tournament();
    assert_eq!(app.tour().stage, Stage::Playing);
    let s = screen(&mut app);
    assert!(s.contains("round 1 of 3") && s.contains("← A"), "{s}");
    assert!(!s.contains("Test Mono"), "names hidden");
    press(&mut app, "n");
    assert!(screen(&mut app).contains("Test Mono"));
    // picks: four in all; u takes one back
    press(&mut app, "hl");
    press(&mut app, "u");
    assert_eq!(app.tour().t.as_ref().unwrap().picks.len(), 1);
    press(&mut app, "lhh");
    assert_eq!(app.tour().stage, Stage::Results);
    let s = screen(&mut app);
    assert!(
        s.contains("the top 10") && s.contains("1st") && s.contains("2nd"),
        "{s}"
    );
    // the bracket
    press(&mut app, "b");
    assert!(screen(&mut app).contains("Round 1 (5 fonts)"));
    press(&mut app, "\x1b");
    assert!(!app.tour().bracket);
    // back to the setup: the result is listed
    press(&mut app, "\x1b");
    assert_eq!(app.tour().stage, Stage::Setup);
    assert!(screen(&mut app).contains("won by Test Mono"));
    assert_eq!(app.tour_past().len(), 1);
    let _ = std::fs::remove_dir_all(crate::model::tournament::dir());
    std::fs::remove_dir_all(dir).unwrap();
}

/// End to end against the real data and commands: re-picks the active
/// preset's current fonts through the studio, which saves (unchanged) and
/// applies them. Run with `NO_RESTART=1` so i3 and Ghostty aren't restarted:
/// `THEME_STUDIO_REAL_SWITCH=1 NO_RESTART=1 cargo test studio_applies_fonts --
/// --ignored --nocapture` (without the first, tests never run the scripts).
#[test]
#[ignore = "runs the real scripts; needs THEME_STUDIO_REAL_SWITCH=1"]
fn studio_applies_fonts_like_font_switch() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let store = Store::locate();
    eprintln!(
        "data {} share {}",
        store.dir.display(),
        store.share.display()
    );
    let config = crate::config::load(&crate::model::paths::studio_config_dir(), false);
    let mut app = App::with_config(store, Start::Active, config).unwrap();
    app.preview.set_headless();
    app.fonts.refresh_installed();
    let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
    for role in 0..3 {
        app.focus_pane(1);
        app.fonts.focus = crate::fonts::types::FontFocus::Slots;
        app.fonts.slot = role;
        let before = app.fonts.preset.fonts[role].clone();
        key(&mut app, KeyCode::Enter); // the list, at the current font
        eprintln!(
            "role {role}: {} {}  highlighted {:?}",
            before.family,
            before.size,
            app.fonts
                .highlighted()
                .map(|i| app.fonts.installed[i].family.clone())
        );
        key(&mut app, KeyCode::Enter); // use it
        eprintln!(
            "  -> {:?}  message {:?}",
            app.fonts.preset.fonts[role], app.messages.message
        );
    }
}

/// True when `file` ends in `.ext` (letter case does not matter).
fn has_extension(file: &str, ext: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// True for a font file that the preview can read (`.ttf` or `.otf`).
fn has_font_extension(file: &str) -> bool {
    has_extension(file, "ttf") || has_extension(file, "otf")
}
