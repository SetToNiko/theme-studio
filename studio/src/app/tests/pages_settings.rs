//! Tests of the Settings page: tabs, behaviour rows, keymap presets, the guide, and failed saves.

use super::state::nav::Pending;
use super::*;
use crate::keymap::action::Action;
use crate::keymap::context::Context;
use std::fmt::Write as _;

#[test]
fn settings_changes_wait_for_save() {
    use crate::settings_page::state::KmRow;
    let store = temp_store("settings-keys");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.on_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
    assert_eq!(app.page, Page::Settings);
    assert_eq!(app.contexts()[0], Context::Settings);
    let row_of = |app: &mut App, r: KmRow| {
        app.show_draft(|a| a.keymap_rows().iter().position(|x| *x == r).unwrap())
    };
    let draft_keys =
        |app: &mut App, c: Context, a: Action| app.show_draft(|x| x.keymap.keys_of(c, a));
    let on_disk =
        |dir: &std::path::Path| crate::config::load::load(&dir.join("studio-config"), false);
    let q = Key::parse_seq("Q", app.keymap.leader).unwrap();
    // Enter on "quit" records new keys: Q, then Enter keeps them, unsaved
    app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Global, Action::Quit));
    press(&mut app, "\n");
    assert!(matches!(app.overlay, Overlay::Capture { .. }));
    press(&mut app, "Q");
    assert!(render(&mut app, 160, 45).contains("Q▏"));
    press(&mut app, "\n");
    assert_eq!(
        draft_keys(&mut app, Context::Global, Action::Quit),
        vec![q.clone()]
    );
    assert_ne!(
        app.keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()],
        "not live yet"
    );
    assert_ne!(
        on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()]
    );
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("● 1 unsaved")
            && screen.contains("s Save")
            && screen.contains("Settings ●"),
        "{screen}"
    );
    // s saves: live and on disk
    press(&mut app, "s");
    assert!(app.settings_page.draft.is_none());
    assert_eq!(
        app.keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()]
    );
    assert_eq!(
        on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
        vec![q]
    );
    // binding pin to y (copy-hex in Colours) warns; D discards
    app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Colours, Action::Pin));
    press(&mut app, "ay\n");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, err)| *err && m.contains("is also")),
        "{:?}",
        app.messages.message
    );
    assert_eq!(draft_keys(&mut app, Context::Colours, Action::Pin).len(), 2);
    press(&mut app, "D");
    assert!(app.settings_page.draft.is_none());
    assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
    // d unbinds, s saves; <leader>u undoes that save, again redoes
    press(&mut app, "ds");
    assert_eq!(
        app.keymap.keys_of(Context::Colours, Action::Pin),
        Vec::<Vec<Key>>::new()
    );
    press(&mut app, " u");
    assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
    press(&mut app, " u");
    assert_eq!(
        app.keymap.keys_of(Context::Colours, Action::Pin),
        Vec::<Vec<Key>>::new()
    );
    // Behaviour tab: the leader, by pressing it
    press(&mut app, "]");
    app.settings_page.bh_sel = 1;
    press(&mut app, "\n,");
    let comma = Key::new(KeyCode::Char(','), KeyModifiers::NONE);
    assert_eq!(app.show_draft(|a| a.keymap.leader), comma);
    assert_ne!(app.keymap.leader, comma, "not live yet");
    // quitting asks, and "save first" saves the settings
    press(&mut app, "Q"); // quit is Q now
    assert!(matches!(app.overlay, Overlay::Confirm(Pending::Quit)));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Unsaved: ") && screen.contains("settings."),
        "{screen}"
    );
    press(&mut app, "s");
    assert!(app.quit);
    assert_eq!(app.keymap.leader, comma);
    assert_eq!(on_disk(&dir).keymap.leader, comma);
    assert_eq!(
        app.keymap.keys_of(Context::Global, Action::SavePalette),
        vec![Key::parse_seq(",sp", comma).unwrap()]
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_behaviour_tab_edits_then_saves() {
    use crate::app::state::nav::BhRow;
    use crate::settings_page::behaviour::behaviour_rows;
    let store = temp_store("settings-behaviour");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    render(&mut app, 160, 45);
    click_on(&mut app, crate::app::state::hit::Target::SettingsTab(1));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Page tabs") && screen.contains("‹ bottom ›"),
        "{screen}"
    );
    let rows = behaviour_rows();
    let on_disk =
        |dir: &std::path::Path| crate::config::load::load(&dir.join("studio-config"), false);
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::Tabs).unwrap();
    press(&mut app, "l");
    assert_eq!(app.show_draft(|a| a.settings.layout.tabs.clone()), "top");
    assert_eq!(app.settings.layout.tabs, "bottom", "not live yet");
    // changing it back leaves nothing unsaved
    render(&mut app, 160, 45);
    let i = app.settings_page.bh_sel;
    click_on(&mut app, crate::app::state::hit::Target::SettingsRow(i));
    assert!(app.settings_page.draft.is_none());
    // again, then the Save button
    press(&mut app, "l");
    render(&mut app, 160, 45);
    click_on(
        &mut app,
        crate::app::state::hit::Target::SettingsButton('s'),
    );
    assert_eq!(app.settings.layout.tabs, "top");
    assert_eq!(on_disk(&dir).settings.layout.tabs, "top");
    // the current pane sizes become the default when saved
    app.layout.sizes.fonts_width = 0.4;
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::SaveSizes).unwrap();
    press(&mut app, "\ns");
    assert_eq!(on_disk(&dir).settings.layout.sizes.fonts_width, 0.4);
    // the Discard button drops changes
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::Tabs).unwrap();
    press(&mut app, "l");
    render(&mut app, 160, 45);
    click_on(
        &mut app,
        crate::app::state::hit::Target::SettingsButton('d'),
    );
    assert!(app.settings_page.draft.is_none());
    assert_eq!(app.settings.layout.tabs, "top");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_history_tab_previews_restores_and_names_versions() {
    use crate::settings_page::diff::DiffLine;
    let store = temp_store("settings-history");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let before = app.editor.palette.get("red");
    app.editor.palette.set("red", Rgb(1, 2, 3));
    app.save(false);
    app.page = Page::Settings;
    render(&mut app, 160, 45);
    click_on(&mut app, crate::app::state::hit::Target::SettingsTab(2));
    // palettes.json in the file list
    app.settings_page.hist_file = crate::settings_page::state::FILES
        .iter()
        .position(|f| *f == "palettes.json")
        .unwrap();
    assert_eq!(
        app.settings_page.tab,
        crate::settings_page::state::Tab::History
    );
    let versions = app.versions();
    assert_eq!(versions.len(), 1);
    // red, and the colours derived from it
    assert!(versions[0].1.summary.starts_with("One: ") && versions[0].1.summary.contains(", red,"));
    // the preview: red as it is now → as it was
    let view = app.diff_view();
    assert!(view.lines.contains(&DiffLine::Changed {
        depth: 2,
        key: "red".into(),
        now: "#010203".into(),
        then: before.hex(),
    }));
    let (now, then) = view.palette.unwrap();
    assert!(now.contains(&("red".into(), Rgb(1, 2, 3))) && then.contains(&("red".into(), before)));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Restore preview") && screen.contains("One: bg_visual_red"),
        "{screen}"
    );
    render(&mut app, 100, 30); // stacked
    // A shows everything
    press(&mut app, "A");
    assert!(app.diff_view().lines.len() > 10);
    // r asks, y restores (and what was there goes to the history)
    press(&mut app, "r");
    assert!(matches!(app.overlay, Overlay::Ask { .. }));
    render(&mut app, 160, 45);
    click_on(&mut app, crate::app::state::hit::Target::ConfirmChoice('y'));
    assert_eq!(app.editor.palette.get("red"), before);
    assert!(!app.dirty());
    assert_eq!(
        Store::at(dir.clone()).load("One").unwrap().get("red"),
        before
    );
    assert_eq!(app.versions().len(), 2);
    // <leader>u undoes the restore
    press(&mut app, " u");
    assert_eq!(app.editor.palette.get("red"), Rgb(1, 2, 3));
    // n names the current file; named versions come first, d deletes them
    press(&mut app, "nmy reds\n");
    let versions = app.versions();
    assert_eq!(versions[0].1.name.as_deref(), Some("my reds"));
    app.settings_page.hist_sel = 1;
    press(&mut app, "d");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Only named"))
    );
    app.settings_page.hist_sel = 0;
    press(&mut app, "dy");
    assert!(app.versions().iter().all(|(_, s)| s.name.is_none()));
    // other files: the list follows
    press(&mut app, "hj");
    assert_eq!(
        crate::settings_page::state::FILES[app.settings_page.hist_file],
        "fonts.json"
    );
    assert_eq!(app.versions(), []);
    assert!(render(&mut app, 160, 45).contains("No earlier versions yet"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_tabs_take_the_keyboard_at_the_list_edge() {
    let store = temp_store("tab-focus");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    // top (the default): up from the first row reaches the tabs
    press(&mut app, "k");
    assert!(app.settings_page.tabs_focused);
    press(&mut app, "l");
    assert_eq!(
        app.settings_page.tab,
        crate::settings_page::state::Tab::Behaviour
    );
    assert!(render(&mut app, 160, 45).contains("back to the list"));
    press(&mut app, "j");
    assert!(!app.settings_page.tabs_focused);
    // the first row is the first group's title (groups fold open / shut)
    assert_eq!(
        crate::settings_page::behaviour::behaviour_rows()[app.settings_page.bh_sel],
        crate::app::state::nav::BhRow::Header("Keys")
    );
    press(&mut app, "jk");
    assert!(!app.settings_page.tabs_focused);
    // tabs at the bottom: down from the last row reaches them
    app.settings.layout.inner_tabs = "bottom".into();
    press(&mut app, "G");
    assert!(!app.settings_page.tabs_focused);
    press(&mut app, "j");
    assert!(app.settings_page.tabs_focused);
    press(&mut app, "l");
    assert_eq!(
        app.settings_page.tab,
        crate::settings_page::state::Tab::History
    );
    press(&mut app, "k");
    assert!(!app.settings_page.tabs_focused);
    let screen = render(&mut app, 160, 45);
    let lines: Vec<&str> = screen.lines().collect();
    assert!(
        lines[lines.len() - 3].contains("Keymaps"),
        "tabs at the bottom:\n{screen}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn preset_browser_previews_loads_creates_and_deletes() {
    use crate::app::state::hit::Target as T;
    let store = temp_store("presets");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("[ Vim ▾ ]"), "{screen}");
    // Enter on the preset button opens the list, on the active preset
    app.settings_page.km_sel = 0;
    press(&mut app, "\n");
    assert_eq!(app.settings_page.browse, Some(0));
    assert!(render(&mut app, 160, 45).contains("Vim (the current keymap)"));
    // j highlights Helix: the left side shows what would change
    press(&mut app, "j");
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Vim → Helix:") && screen.contains("differ"),
        "{screen}"
    );
    assert!(
        screen.contains("redo") && !screen.contains("main page"),
        "only differences:\n{screen}"
    );
    press(&mut app, "A");
    assert!(
        render(&mut app, 160, 45).contains("main page"),
        "everything"
    );
    // n makes a new preset from the highlighted one
    press(&mut app, "nMy Helix\n");
    let names = |app: &mut App| {
        app.show_draft(|a| a.presets.iter().map(|p| p.name.clone()).collect::<Vec<_>>())
    };
    assert!(names(&mut app).contains(&"My Helix".to_string()));
    let mine = names(&mut app)
        .iter()
        .position(|n| n == "My Helix")
        .unwrap();
    assert_eq!(app.settings_page.browse, Some(mine));
    // Enter loads it (unsaved); s saves
    press(&mut app, "\n");
    assert_eq!(app.settings_page.browse, None);
    assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "My Helix");
    assert_eq!(app.keymap.name, "Vim", "not live yet");
    press(&mut app, "s");
    assert_eq!(app.keymap.name, "My Helix");
    let saved = crate::config::load::load(&dir.join("studio-config"), false);
    assert_eq!(saved.keymap.name, "My Helix");
    // shipped presets can't be deleted; yours can (asks first)
    app.settings_page.km_sel = 0;
    press(&mut app, "\n");
    assert_eq!(
        app.settings_page.browse,
        Some(mine),
        "opens on the active one"
    );
    press(&mut app, "ggd"); // Helix keys are live now: gg, ge
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("can't be deleted"))
    );
    press(&mut app, "ge");
    assert_eq!(app.settings_page.browse, Some(mine));
    press(&mut app, "dy");
    assert!(!names(&mut app).contains(&"My Helix".to_string()));
    assert_eq!(
        app.show_draft(|a| a.keymap.name.clone()),
        "Vim",
        "the active one went: Vim"
    );
    press(&mut app, "s");
    assert!(
        !crate::config::load::load(&dir.join("studio-config"), false)
            .presets
            .iter()
            .any(|p| p.name == "My Helix")
    );
    // the mouse: a click highlights, a second loads
    render(&mut app, 160, 45);
    click_on(&mut app, T::PresetRow(2));
    assert_eq!(app.settings_page.browse, Some(2));
    render(&mut app, 160, 45);
    click_on(&mut app, T::PresetRow(2));
    assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "Emacs");
    press(&mut app, "D");
    // Esc closes the list first, then leaves the page
    app.settings_page.km_sel = 0;
    press(&mut app, "\n\x1b");
    assert_eq!((app.settings_page.browse, app.page), (None, Page::Settings));
    press(&mut app, "\x1b");
    assert_eq!(app.page, Page::Main);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_groups_fold_and_only_names_are_underlined() {
    use crate::app::state::hit::Target as T;
    use crate::app::state::nav::BhRow;
    use crate::settings_page::behaviour::visible_behaviour_rows;
    use crate::settings_page::state::KmRow;
    let mut app = test_app(temp_store("fold"), Start::Named("One".into()));
    app.page = Page::Settings;
    // Keymaps: Enter on a group's title closes it, again opens it
    let n = app.keymap_rows().len();
    let g = app
        .keymap_rows()
        .iter()
        .position(|r| *r == KmRow::Header(Context::Global))
        .unwrap();
    app.settings_page.km_sel = g;
    press(&mut app, "\n");
    assert!(app.keymap_rows().len() < n);
    assert!(render(&mut app, 160, 45).contains("▸ Everywhere"));
    press(&mut app, "l");
    assert_eq!(app.keymap_rows().len(), n);
    // the cursor stops on titles but skips blank lines
    press(&mut app, "k");
    assert_eq!(app.keymap_rows()[app.settings_page.km_sel], KmRow::Preset);
    // Behaviour: a click on the selected title folds it
    app.settings_page.tab = crate::settings_page::state::Tab::Behaviour;
    render(&mut app, 160, 45);
    let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
    let hover = rows
        .iter()
        .position(|r| *r == BhRow::Header("Hover (everything clickable)"))
        .unwrap();
    app.settings_page.bh_sel = hover;
    render(&mut app, 160, 45);
    click_on(&mut app, T::SettingsRow(hover));
    assert!(
        app.settings_page
            .bh_closed
            .contains("Hover (everything clickable)")
    );
    assert!(visible_behaviour_rows(&app.settings_page.bh_closed).len() < rows.len());
    // hover: only the option's name is underlined, not the row
    let mut t = Terminal::new(TestBackend::new(160, 45)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let leader = visible_behaviour_rows(&app.settings_page.bh_closed)
        .iter()
        .position(|r| *r == BhRow::Leader)
        .unwrap();
    let r = app.hits.rect_of(T::SettingsRow(leader)).unwrap();
    mouse(&mut app, MouseEventKind::Moved, r.x + 5, r.y);
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    let under = |x: u16| {
        buf[(x, r.y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    };
    let row: String = (r.x..r.right())
        .map(|x| buf[(x, r.y)].symbol().to_string())
        .collect();
    let start = r.x + u16::try_from(row.find("Leader").unwrap()).unwrap();
    assert!(under(start) && under(start + 10), "the name");
    assert!(
        !under(r.x) && !under(start + 45) && !under(start + 52),
        "not the padding or value:\n{row}"
    );
    std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
}

#[test]
fn the_readme_tab_renders_scrolls_and_jumps_between_headings() {
    let store = temp_store("readme");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    // the guide isn't next to the temp configs: give the tab its text
    let mut md = String::from("# Guide\n\nFirst part.\n\n");
    for i in 0..60 {
        writeln!(md, "- line {i}").unwrap();
    }
    md += "\n## Second\n\n| a | b |\n| - | - |\n| `x` | y |\n";
    app.settings_page.readme = Some((None, Ok(md)));
    app.page = Page::Settings;
    press(&mut app, "]]]");
    assert_eq!(
        app.settings_page.tab,
        crate::settings_page::state::Tab::Readme
    );
    let screen = render(&mut app, 120, 30);
    assert!(
        screen.contains("README") && screen.contains("Guide") && screen.contains("• line 0"),
        "{screen}"
    );
    assert!(
        screen.contains("} /{ headings") || screen.contains("}/{ headings"),
        "{screen}"
    );
    press(&mut app, "}");
    let second = app.settings_page.readme_scroll;
    assert!(second > 60);
    let screen = render(&mut app, 120, 30);
    assert!(
        screen.lines().nth(3).is_some_and(|l| l.contains("Second")),
        "{screen}"
    );
    press(&mut app, "{");
    assert_eq!(app.settings_page.readme_scroll, 0);
    // at the top, up goes on to the tab strip
    press(&mut app, "k");
    assert!(app.settings_page.tabs_focused);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn behaviour_tab_sets_up_prefetching() {
    use crate::app::state::nav::BhRow;
    use crate::settings_page::behaviour::behaviour_rows;
    let store = temp_store("prefetch-rows");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.settings_page.bh_closed.clear();
    let at = |r: BhRow| behaviour_rows().iter().position(|x| *x == r).unwrap();
    let run = |app: &mut App, r: BhRow| {
        app.with_draft(|a| {
            a.settings_page.bh_sel = at(r);
            a.run_behaviour_tab(Action::Confirm);
        });
    };
    // prefetching: off by default, monospace chosen; edits wait for Save
    assert!(!app.settings.fonts_page.prefetch);
    run(&mut app, BhRow::Prefetch);
    run(&mut app, BhRow::PrefetchCategory(5));
    assert!(!app.settings.fonts_page.prefetch, "not live before Save");
    app.settings_save();
    assert!(app.settings.fonts_page.prefetch);
    assert_eq!(
        app.settings.fonts_page.prefetch_categories,
        ["monospace", "nerd font"]
    );
    // with the catalogue, each category shows what it takes
    let nerd = crate::system::download::types::Source::Nerd {
        asset: "Hack".into(),
        url: String::new(),
        size: 40_000_000,
    };
    let mut hack = crate::system::download::types::Entry {
        id: "hack".into(),
        family: "Hack".into(),
        category: "nerd font".into(),
        license: String::new(),
        weights: vec![400],
        italic: false,
        variable: false,
        source: nerd,
        subset: "latin".into(),
    };
    app.dl.catalogue = Some(crate::system::download::types::Catalogue {
        nerd: vec![hack.clone()],
        ..Default::default()
    });
    assert_eq!(
        app.behaviour_value(BhRow::PrefetchCategory(5)),
        "yes  (1 font: 40.0 MB to fetch, about 2.5 MB kept)"
    );
    hack.category = "monospace".into();
    hack.source = crate::system::download::types::Source::Google;
    app.dl.catalogue.as_mut().unwrap().google = vec![hack];
    assert_eq!(
        app.behaviour_value(BhRow::PrefetchCategory(0)),
        "yes  (1 font, about 100 KB)"
    );
    // Clear asks first (the answer isn't given here: it would delete
    // the real folder)
    run(&mut app, BhRow::ClearPreviews);
    assert!(matches!(
        app.overlay,
        Overlay::Ask {
            then: crate::app::state::nav::Deferred::ClearPreviews,
            ..
        }
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn behaviour_cursor_keeps_the_swatch() {
    use crate::app::state::nav::{BhRow, LookId};
    use crate::settings_page::behaviour::visible_behaviour_rows;
    use ratatui::style::Modifier;
    let store = temp_store("cursor-swatch");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    app.settings_page.tab = crate::settings_page::state::Tab::Behaviour;
    app.settings_page.bh_closed.clear();
    let row = BhRow::HighlightColour(LookId::Hover);
    app.settings_page.bh_sel = visible_behaviour_rows(&app.settings_page.bh_closed)
        .iter()
        .position(|r| *r == row)
        .unwrap();
    let mut t = Terminal::new(TestBackend::new(160, 60)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    let label = App::behaviour_label(row);
    let y = (0..60)
        .find(|&y| {
            let s: String = (0..160).map(|x| buf[(x, y)].symbol().to_string()).collect();
            s.contains(&label) && s.contains("██")
        })
        .expect("the row is drawn");
    let cells: Vec<_> = (0..160).map(|x| buf[(x, y)].clone()).collect();
    let reversed = |c: &ratatui::buffer::Cell| c.modifier.contains(Modifier::REVERSED);
    // the swatch keeps its colour and isn't reversed
    let swatch = cells.iter().find(|c| c.symbol() == "█").unwrap();
    assert!(!reversed(swatch));
    assert_eq!(
        swatch.fg,
        app.theme_colour(&app.settings.hover.look.highlight_colour)
    );
    // the label is reversed, the padding after it isn't
    let start = (0..160).find(|&x| buf[(x, y)].symbol() == "H").unwrap() as usize;
    assert!(reversed(&cells[start]));
    let end = start + label.chars().count();
    assert!(!reversed(&cells[end + 1]), "padding after the label");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_failed_settings_save_keeps_the_draft_and_the_old_live_values() {
    let store = temp_store("save-fails");
    // a file where the config folder should be: every write fails
    let config_dir = store.dir.join("config-is-a-file");
    std::fs::write(&config_dir, "not a folder").unwrap();
    let config = crate::config::load::Loaded {
        keymap: crate::keymap::map::Keymap::vim(),
        presets: crate::keymap::map::Keymap::builtin(),
        settings: crate::config::settings::Settings::default(),
        warnings: Vec::new(),
        dir: config_dir,
    };
    let mut app = App::with_config(store, Start::Named("One".into()), config).unwrap();
    app.preview.set_headless();
    let live = app.settings.font_list.hide_bitmap;
    app.with_draft(|a| a.settings.font_list.hide_bitmap = !live);
    app.settings_save();
    assert!(app.settings_page.draft.is_some(), "the draft is kept");
    assert_eq!(
        app.settings.font_list.hide_bitmap, live,
        "live is unchanged"
    );
    assert_eq!(app.draft_changes(), 1);
    let (m, is_error) = app.messages.message.clone().unwrap();
    assert!(is_error && m.starts_with("Saving:"), "{m}");
}
