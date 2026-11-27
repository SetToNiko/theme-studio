//! App's methods that install and remove fonts on the Download fonts page
//! (F2): install results, the family fill after a pick, the Nerd Font offer,
//! and removal.
//!
//! This is another `impl App { ... }` block. The page rows, polling and key
//! handling stay in `fonts_dl_actions.rs`.

use std::sync::mpsc::Sender;

use serde_json::Value;

use crate::app::app_state::App;
use crate::app::state::nav::{Deferred, Overlay};
use crate::fonts::pages::download::{DlRow, DlStatus, norm};
use crate::system::download;
use crate::system::download::types::{Entry, Msg, Source};

impl App {
    /// An install of job `id` ended. Records it, and tells the user when the
    /// font cache refresh failed (`warning`).
    pub(super) fn on_installed(
        &mut self,
        id: String,
        result: Result<(String, Value), String>,
        warning: Option<String>,
    ) {
        let installed = result.is_ok();
        let family = self
            .dl
            .jobs
            .get(&id)
            .map_or_else(|| id.clone(), |job| job.0.clone());
        self.finish_install(id, result);
        if let (true, Some(w)) = (installed, warning) {
            self.messages.error(format!(
                "Installed {family}, but the font cache refresh failed, so the font may show up late. Details: {w}"
            ));
        }
    }

    /// An install finished: record it (and fill the roles that asked for it),
    /// or show why it failed.
    fn finish_install(&mut self, id: String, result: Result<(String, Value), String>) {
        let fill = match &self.dl.fill_after {
            Some((after, ..)) if *after == id => self.dl.fill_after.take(),
            _ => None,
        };
        let family = self.dl.jobs.remove(&id).map_or(id, |j| j.0);
        match result {
            Ok((name, entry)) => {
                self.record_install(&name, entry);
                if let Some((_, group, roles)) = fill {
                    let fill = self.fonts.fill_roles(&group, &roles);
                    self.apply_fill(fill);
                }
            }
            Err(e) => self
                .messages
                .error(format!("Downloading {family} failed: {e}")),
        }
    }

    /// Where background work reports back.
    pub fn dl_sender(&self) -> Sender<Msg> {
        self.dl.sender()
    }

    fn record_install(&mut self, name: &str, entry: Value) {
        let path = self.installed_list_path();
        let mut list = download::list::read_list(&path);
        list.insert(name.to_string(), entry);
        self.dl.listed = None;
        match download::list::write_list(&path, list) {
            Ok(()) => self.messages.info(format!(
                "Installed {name}: it's in the font list now; a rebuild keeps it (listed in {})",
                path.display()
            )),
            Err(e) => self
                .messages
                .error(format!("Installed {name}, but {}: {e:#}", path.display())),
        }
        self.fonts.refresh_installed();
    }

    /// After filling roles from a family (`F`, `r` on a role, a tournament
    /// result's 1 / 2 / 3): applies what changed, and offers the family's
    /// Nerd Font when it has no fitting face for a role.
    pub fn after_fill(&mut self, fill: crate::fonts::pick::Fill) {
        let offer = if fill.missing.is_empty() {
            None
        } else {
            self.nerd_entry_for(&fill.group)
        };
        let (group, missing) = (fill.group.clone(), fill.missing.clone());
        self.apply_fill(fill);
        if let Some(e) = offer {
            let size = match &e.source {
                Source::Nerd { size, .. } => format!(" ({})", crate::system::prefetch::size(*size)),
                _ => String::new(),
            };
            self.overlay = Overlay::Ask {
                question: format!(
                    "{group} has no {} installed. Download {}{size}?",
                    crate::fonts::pick::faces(&missing),
                    e.family
                ),
                then: Deferred::NerdForRoles {
                    id: e.id,
                    group,
                    roles: missing,
                },
            };
        }
    }

    /// Saves and applies a fill if a role changed; otherwise says why not.
    fn apply_fill(&mut self, fill: crate::fonts::pick::Fill) {
        if fill.changed {
            self.apply_picked_fonts(&fill.msg);
        } else {
            self.messages.info(fill.msg);
        }
    }

    /// The Nerd Fonts release's archive for a family (`JetBrains Mono` →
    /// `JetBrainsMono.tar.xz`). Starts loading the catalogue if it isn't yet
    /// (asking again later finds it).
    fn nerd_entry_for(&mut self, group: &str) -> Option<Entry> {
        if self.dl.catalogue.is_none() && !self.dl.loading {
            self.dl.start_catalogue_load();
        }
        let key = norm(crate::fonts::pick::base_family(group));
        self.dl
            .catalogue
            .as_ref()?
            .nerd
            .iter()
            .find(|e| match &e.source {
                Source::Nerd { asset, .. } => norm(asset) == key,
                _ => false,
            })
            .cloned()
    }

    /// Yes to the offer: downloads the Nerd Font, then fills the roles.
    pub fn install_nerd_for_roles(&mut self, id: String, group: String, roles: Vec<usize>) {
        let entry = self
            .dl
            .catalogue
            .as_ref()
            .and_then(|c| c.nerd.iter().find(|e| e.id == id).cloned());
        if let Some(e) = entry {
            self.dl.fill_after = Some((id, group, roles));
            self.dl_install_entry(e);
        }
    }

    pub fn dl_install(&mut self) {
        if let Some(e) = self.dl_selected() {
            self.dl_install_entry(e);
        }
    }

    /// Installs a catalogue font (unless it is already, or on its way).
    pub fn dl_install_entry(&mut self, e: Entry) {
        let listed = self.dl_listed();
        match self.dl_status(&e, &listed) {
            DlStatus::Downloading(..) => self.messages.info(format!("{} is downloading", e.family)),
            DlStatus::Listed => self
                .messages
                .info(format!("{} is installed already", e.family)),
            _ => {
                self.dl.jobs.insert(e.id.clone(), (e.family.clone(), 0, 0));
                self.messages.info(format!("Downloading {}…", e.family));
                download::install::install(
                    e,
                    crate::model::paths::installed_fonts_dir(),
                    self.dl.sender(),
                );
            }
        }
    }

    /// The list name of the highlighted row, or `None` when no row is highlighted.
    fn dl_selected_name(&mut self) -> Option<String> {
        match self.dl_rows().get(self.dl.sel) {
            Some(DlRow::Listed(n)) => Some(n.clone()),
            Some(DlRow::Entry(i)) => self.dl_entries().get(*i).map(Self::list_name),
            None => None,
        }
    }

    /// Removes a font from the list and its downloaded files.
    pub fn dl_remove(&mut self) {
        let Some(name) = self.dl_selected_name() else {
            return;
        };
        let path = self.installed_list_path();
        let mut list = download::list::read_list(&path);
        let Some(entry) = list.remove(&name) else {
            return self
                .messages
                .info(format!("{name} wasn't installed by the studio"));
        };
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let result =
            download::install::remove_files(&crate::model::paths::installed_fonts_dir(), &id)
                .and_then(|w| download::list::write_list(&path, list).map(|()| w));
        self.dl.listed = None;
        self.report_removal(&name, result);
        self.fonts.refresh_installed();
        let n = self.dl_rows().len();
        self.dl.sel = self.dl.sel.min(n.saturating_sub(1));
    }

    /// Tells the user how removing font `name` went: done, done with a font
    /// cache warning, or failed.
    fn report_removal(&mut self, name: &str, result: anyhow::Result<Option<String>>) {
        match result {
            Ok(None) => self
                .messages
                .info(format!("Removed {name} (a rebuild drops it from Nix too)")),
            Ok(Some(w)) => self.messages.error(format!(
                "Removed {name} (a rebuild drops it from Nix too), but the font cache refresh failed, so the font may show up for a while. Details: {w}"
            )),
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::startup::Start;
    use crate::app::state::nav::Overlay;
    use crate::app::test_support::*;

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
}
