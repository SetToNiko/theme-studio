//! App's methods that install and remove fonts on the Download fonts page
//! (F2): install results, the family fill after a pick, the Nerd Font offer,
//! and removal.
//!
//! This is another `impl App { ... }` block. The page rows, polling and key
//! handling stay in `fonts_dl_actions.rs`.

use std::sync::mpsc::Sender;

use serde_json::Value;

use crate::app::App;
use crate::app::state::nav::{Deferred, Overlay};
use crate::fonts::download_page::{DlRow, DlStatus, norm};
use crate::system::download::{self, Entry, Msg, Source};

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
