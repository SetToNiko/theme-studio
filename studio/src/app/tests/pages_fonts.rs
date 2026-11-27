//! Tests of the Fonts page: roles, families, presets, and sizes.

use crate::app::startup::Start;
use crate::app::state::nav::Pending;
use crate::app::state::nav::{Focus, Overlay, Side};
use crate::app::test_support::*;
use crate::model::store::to_jq_string;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one walk through the fonts tab (edit, save, open a preset); each step uses the state the step before left"
)]
fn fonts_tab_edits_saves_and_opens_presets() {
    let store = temp_store("fonts");
    let dir = store.dir.clone();
    std::fs::write(
        dir.join("fonts.json"),
        "{\n  \"Base\": {\n    \"mono\": {\n      \"family\": \"A\",\n      \"size\": 14\n    },\n    \"ui\": {\n      \"family\": \"B\",\n      \"size\": 9\n    }\n  }\n}\n",
    )
    .unwrap();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    app.fonts.open("Base").unwrap();
    press(&mut app, " 1"); // the font roles
    assert_eq!(app.side, Side::Fonts);
    app.fonts.set_installed(vec![
        crate::fonts::types::Installed {
            family: "A".into(),
            mono: true,
            nerd: true,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "B".into(),
            mono: false,
            nerd: false,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "C Mono".into(),
            mono: true,
            nerd: false,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "C Nerd Font Propo".into(),
            mono: false,
            nerd: true,
            bitmap: false,
            special: false,
        },
    ]);
    // A is no Nerd Font Mono: the role says so, with the fix key
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("⚠ not a Nerd Font Mono · r fixes"),
        "{screen}"
    );
    // Icons (third role): Enter lists only the Nerd Font Propo fonts
    press(&mut app, "jj\n");
    assert_eq!(app.fonts.picking, Some(2));
    assert_eq!(app.fonts.rows().len(), 1);
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Nerd Font Propo]") && screen.contains("picking Icons"),
        "{screen}"
    );
    // the chips that would widen it are locked (and hidden)
    press(&mut app, "n");
    assert_eq!(app.fonts.rows().len(), 1, "n is locked while picking");
    // Enter picks it (saved and applied: font-switch.sh isn't in the
    // temp folder, so applying reports it), two sizes bigger
    press(&mut app, "\n");
    assert_eq!(app.fonts.picking, None);
    assert_eq!(app.fonts.filters.need, None, "the user's filters are back");
    assert!(!app.fonts.dirty(), "saved straight away");
    press(&mut app, "ll");
    assert_eq!(app.fonts.preset.fonts[2].family, "C Nerd Font Propo");
    // the sizes are the theme's, not the preset's: the new theme's 11, +2
    assert_eq!(app.fonts.preset.fonts[2].size, 13);
    // number keys: 2 opens the list, 1 goes back
    press(&mut app, " 2");
    assert_eq!(app.fonts.focus, crate::fonts::types::FontFocus::List);
    press(&mut app, " 1");
    // a size isn't the preset's: it stays saved, and opening the preset
    // again doesn't ask, and keeps the sizes
    assert!(!app.fonts.dirty());
    press(&mut app, " ofBase\n");
    assert!(!matches!(
        app.overlay,
        Overlay::Confirm(Pending::OpenFonts(_))
    ));
    assert_eq!(app.fonts.preset.fonts[2].size, 13);
    // <leader>sf saves the preset in place, in jq's format, families only
    press(&mut app, " sf");
    assert!(!app.fonts.dirty());
    let text = std::fs::read_to_string(dir.join("fonts.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], 4, "saved as version 4");
    assert_eq!(
        v["presets"]["Base"]["icons"],
        serde_json::json!({"family": "C Nerd Font Propo"})
    );
    assert_eq!(text, to_jq_string(&v));
    // the palette tab didn't change, and its keys still work after F1
    assert!(!app.dirty());
    press(&mut app, " 5");
    assert_eq!((app.side, app.focus), (Side::Colours, Focus::Square));
    assert!(app.layout.picker_visible);
    // the Fonts tab draws at every size (no image support in tests)
    let screen = render(&mut app, 160, 45);
    for t in [
        "¹Fonts",
        "²Installed fonts",
        "F2 Fonts",
        "Sample",
        "C Nerd Font Propo",
    ] {
        assert!(screen.contains(t), "missing {t}");
    }
    for w in (1..=200).step_by(7) {
        for h in (1..=70).step_by(5) {
            render(&mut app, w, h);
        }
    }
}

#[test]
fn a_family_fills_every_role() {
    use crate::fonts::types::{Installed, Row};
    let store = temp_store("family");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    let f = |family: &str, mono: bool, nerd: bool| Installed {
        family: family.into(),
        mono,
        nerd,
        bitmap: false,
        special: false,
    };
    app.fonts.set_installed(vec![
        f("JetBrains Mono", true, false),
        f("JetBrains Mono NL", true, false),
        f("JetBrainsMono Nerd Font", true, true),
        f("JetBrainsMono Nerd Font Mono", true, true),
        f("JetBrainsMono Nerd Font Propo", false, true),
        f("Inter", false, false),
    ]);
    press(&mut app, " 2");
    // the family's group row, then F
    let g = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Group { name, .. } if name.starts_with("JetBrains")))
        .unwrap();
    app.fonts.list_sel = g;
    press(&mut app, "F");
    let fams: Vec<&str> = app
        .fonts
        .preset
        .fonts
        .iter()
        .map(|f| f.family.as_str())
        .collect();
    assert_eq!(
        fams,
        [
            "JetBrainsMono Nerd Font Mono",
            "JetBrainsMono Nerd Font Propo",
            "JetBrainsMono Nerd Font Propo"
        ]
    );
    assert!(!app.fonts.dirty(), "saved straight away");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("From JetBrains")),
        "{:?}",
        app.messages.message
    );
    // a font in no group: the family is its own name, so Inter for all
    let i = app
        .fonts
        .installed
        .iter()
        .position(|f| f.family == "Inter")
        .unwrap();
    let inter = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Font { index, .. } if *index == i))
        .unwrap();
    app.fonts.focus = crate::fonts::types::FontFocus::List;
    app.fonts.list_sel = inter;
    press(&mut app, "F");
    let fams: Vec<&str> = app
        .fonts
        .preset
        .fonts
        .iter()
        .map(|f| f.family.as_str())
        .collect();
    // Monospace and Icons keep theirs: Inter has no Nerd Font faces (the
    // test catalogue has no Nerd Font of it either, so nothing is offered)
    assert_eq!(
        fams,
        [
            "JetBrainsMono Nerd Font Mono",
            "Inter",
            "JetBrainsMono Nerd Font Propo"
        ]
    );
    assert!(!matches!(app.overlay, Overlay::Ask { .. }));
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Monospace, Icons kept")),
        "{:?}",
        app.messages.message
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_family_without_fitting_faces_offers_its_nerd_font() {
    use crate::fonts::types::{Installed, Row};
    use crate::system::download::types::{Catalogue, Entry, Source};
    let store = temp_store("offer");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    app.fonts.set_installed(vec![Installed {
        family: "Fira Code".into(),
        mono: true,
        nerd: false,
        bitmap: false,
        special: false,
    }]);
    app.dl.catalogue = Some(Catalogue {
        nerd: vec![Entry {
            id: "nerd-firacode".into(),
            family: "FiraCode Nerd Font".into(),
            category: "nerd font".into(),
            license: String::new(),
            weights: Vec::new(),
            italic: false,
            variable: false,
            source: Source::Nerd {
                asset: "FiraCode".into(),
                url: "https://x/FiraCode.tar.xz".into(),
                size: 12_000_000,
            },
            subset: String::new(),
        }],
        ..Default::default()
    });
    press(&mut app, " 2");
    app.fonts.list_sel = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Font { .. }))
        .unwrap();
    let before = app.fonts.preset.fonts.clone();
    press(&mut app, "F");
    assert_eq!(
        app.fonts.preset.fonts, before,
        "nothing fits: every role kept"
    );
    match &app.overlay {
        Overlay::Ask { question, then } => {
            assert_eq!(
                question,
                "Fira Code has no Nerd Font Mono, proportional or Nerd Font Propo face installed. Download FiraCode Nerd Font (12.0 MB)?"
            );
            assert_eq!(
                *then,
                crate::app::state::nav::Deferred::NerdForRoles {
                    id: "nerd-firacode".into(),
                    group: "Fira Code".into(),
                    roles: vec![0, 1, 2],
                }
            );
        }
        _ => panic!("no offer"),
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sizes_belong_to_the_theme_and_are_saved_when_the_keys_stop() {
    let store = temp_store("sizes");
    let dir = store.dir.clone();
    std::fs::write(
        dir.join("fonts.json"),
        r#"{"version": 4, "presets": {"Base": {"monospace": {"family": "A"}, "proportional": {"family": "B"}, "icons": {"family": "C"}}, "Other": {"monospace": {"family": "D"}, "proportional": {"family": "E"}, "icons": {"family": "F"}}}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("themes.json"),
        r#"{"version": 2, "themes": {"T": {"palette": "One", "fonts": "Base", "sizes": {"monospace": 14, "proportional": 9, "icons": 9}}}}"#,
    )
    .unwrap();
    let mut app = test_app(store, Start::Named("T".into()));
    app.fonts.store.dir = dir.clone();
    app.themes.dir = dir.clone();
    app.open_theme("T");
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 9], "the theme's sizes");
    // l on Icons: the size changes at once, and is saved and applied
    // once the keys stop (the run loop calls apply_sizes after 400 ms)
    press(&mut app, " 1");
    app.fonts.slot = 2;
    press(&mut app, "ll");
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 11]);
    assert!(app.fonts.sized_at.is_some());
    assert!(!app.fonts.dirty(), "the preset isn't changed by a size");
    app.apply_sizes();
    assert!(app.fonts.sized_at.is_none());
    assert_eq!(app.themes.get("T").unwrap().sizes, Some([14, 9, 11]));
    assert!(!app.theme_dirty(), "saved: nothing left unsaved");
    // the preset file has no sizes
    let f = std::fs::read_to_string(dir.join("fonts.json")).unwrap();
    assert!(!f.contains("size"));
    // swapping in another preset keeps the theme's sizes
    app.fonts.open("Other").unwrap();
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 11]);
    assert_eq!(app.fonts.preset.fonts[0].family, "D");
    std::fs::remove_dir_all(dir).unwrap();
}
