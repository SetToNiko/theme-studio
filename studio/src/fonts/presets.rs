//! Font presets and fonts.json: the Monospace, Proportional and Icons
//! families of a preset, reading and writing them, and the files around
//! fonts.json (the data folder).

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value, json};

use super::types::ROLES;

/// The keys before version 3, still read.
pub const OLD_ROLES: [&str; 3] = ["mono", "ui", "bar"];
/// What each role is used for (font-switch.sh).
pub const ROLE_USE: [&str; 3] = [
    "terminal, nvim, lock screen",
    "rofi, GTK apps, sans-serif text",
    "i3 window titles and the bar",
];

/// A role's entry in a preset or an offset file: the new key, else the old.
fn role_entry(obj: &Map<String, Value>, role: usize) -> Option<&Value> {
    obj.get(ROLES[role]).or_else(|| obj.get(OLD_ROLES[role]))
}

/// A preset without its roles' sizes (fonts.json version 4).
pub fn drop_sizes(obj: Map<String, Value>) -> Map<String, Value> {
    obj.into_iter()
        .map(|(k, mut v)| {
            if ROLES.contains(&k.as_str())
                && let Some(o) = v.as_object_mut()
            {
                o.remove("size");
            }
            (k, v)
        })
        .collect()
}

/// A preset or offset file with the old role keys renamed (in place, so
/// the order stays).
pub fn rename_roles(obj: Map<String, Value>) -> Map<String, Value> {
    obj.into_iter()
        .map(|(k, v)| match OLD_ROLES.iter().position(|o| *o == k) {
            Some(r) => (ROLES[r].to_string(), v),
            None => (k, v),
        })
        .collect()
}

/// A role's font. The family comes from the font preset; the size from the
/// theme (themes.json `sizes`), so switching presets keeps the sizes.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub family: String,
    pub size: u32,
}

/// Sizes when neither the theme nor (an older) preset has any.
pub const DEFAULT_SIZES: [u32; 3] = [14, 11, 11];

/// A font preset: the Monospace, Proportional and Icons fonts' families.
/// (The sizes held here are the theme's, for drawing and applying; they
/// aren't saved with the preset.)
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub fonts: [Font; 3],
    /// Other keys in the preset (kept as they are when saving).
    extra: Vec<(String, Value)>,
}

impl Preset {
    /// Reads a preset (either key names); a missing Icons font falls back
    /// to the Proportional one, like font-switch.sh. Sizes are read when
    /// the preset still has them (before fonts.json version 4), else the
    /// defaults; the theme's sizes replace them (`set_sizes`).
    pub fn from_json(obj: &Map<String, Value>) -> Result<Preset> {
        let font = |role: usize| -> Option<Font> {
            let f = role_entry(obj, role)?;
            Some(Font {
                family: f.get("family")?.as_str()?.to_string(),
                size: f
                    .get("size")
                    .and_then(Value::as_u64)
                    .map_or(DEFAULT_SIZES[role], |s| {
                        u32::try_from(s).unwrap_or(DEFAULT_SIZES[role])
                    }),
            })
        };
        let mono = font(0).ok_or_else(|| anyhow!("no monospace font"))?;
        let ui = font(1).ok_or_else(|| anyhow!("no proportional font"))?;
        let bar = font(2).unwrap_or_else(|| ui.clone());
        let extra = obj
            .iter()
            .filter(|(k, _)| !ROLES.contains(&k.as_str()) && !OLD_ROLES.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Preset {
            fonts: [mono, ui, bar],
            extra,
        })
    }

    /// The preset as fonts.json keeps it: families only.
    pub fn to_json(&self) -> Map<String, Value> {
        let mut m = Map::new();
        for (role, f) in ROLES.iter().zip(&self.fonts) {
            m.insert(role.to_string(), json!({ "family": f.family }));
        }
        for (k, v) in &self.extra {
            m.insert(k.clone(), v.clone());
        }
        m
    }

    pub fn sizes(&self) -> [u32; 3] {
        [0, 1, 2].map(|r| self.fonts[r].size)
    }

    pub fn set_sizes(&mut self, sizes: [u32; 3]) {
        for (f, s) in self.fonts.iter_mut().zip(sizes) {
            f.size = s;
        }
    }

    /// The same fonts (families and other keys), whatever the sizes.
    pub fn same_fonts(&self, other: &Preset) -> bool {
        self.extra == other.extra
            && self
                .fonts
                .iter()
                .zip(&other.fonts)
                .all(|(a, b)| a.family == b.family)
    }

    /// What a new preset starts from.
    pub fn default_preset() -> Preset {
        let f = |family: &str, size| Font {
            family: family.to_string(),
            size,
        };
        Preset {
            fonts: [
                f("JetBrainsMono Nerd Font", 14),
                f("JetBrainsMono Nerd Font", 11),
                f("JetBrainsMono Nerd Font", 11),
            ],
            extra: Vec::new(),
        }
    }
}

/// fonts.json (in the data folder) and the files around it.
pub struct FontStore {
    /// The data folder.
    pub dir: PathBuf,
    /// The share folder (passed on to font-switch).
    pub share: PathBuf,
}

impl FontStore {
    fn path(&self) -> PathBuf {
        self.dir.join("fonts.json")
    }

    /// All presets, in file order (either layout of fonts.json).
    pub fn load_all(&self) -> Result<Map<String, Value>> {
        let text = std::fs::read_to_string(self.path())
            .with_context(|| format!("reading {}", self.path().display()))?;
        let v: Value = serde_json::from_str(&text)?;
        crate::model::store::data_map(v, "presets")
            .ok_or_else(|| anyhow!("fonts.json has no presets"))
    }

    /// Brings fonts.json up to the current version (3: the new role keys;
    /// 4: no sizes, they're the theme's), and the host offset files
    /// (configs/hosts/*/fonts.json) with it. Run after themes.json's
    /// migration, which takes the sizes from here first.
    pub fn migrate(&self) -> Result<Option<u64>> {
        let done = crate::model::store::migrate_data_file(&self.path(), "presets", |presets| {
            presets
                .into_iter()
                .map(|(name, p)| match p {
                    Value::Object(o) => (name, Value::Object(drop_sizes(rename_roles(o)))),
                    other => (name, other),
                })
                .collect()
        })?;
        self.migrate_offsets()?;
        Ok(done)
    }

    /// The host offset files with the old role keys get the new ones (not
    /// kept in the history: it's by file name, and fonts.json is taken).
    fn migrate_offsets(&self) -> Result<()> {
        // a hosts folder beside fonts.json (the old layout kept them
        // together; font-switch reads both key names either way)
        let Ok(hosts) = std::fs::read_dir(self.dir.join("hosts")) else {
            return Ok(());
        };
        for host in hosts.flatten() {
            let path = host.path().join("fonts.json");
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if !OLD_ROLES.iter().any(|k| obj.contains_key(*k)) {
                continue;
            }
            let new = crate::model::store::to_jq_string(&Value::Object(rename_roles(obj)));
            std::fs::write(&path, new).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(())
    }

    pub fn load(&self, name: &str) -> Result<Preset> {
        let all = self.load_all()?;
        let obj = all
            .get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("no font preset named '{name}'"))?;
        Preset::from_json(obj).with_context(|| format!("font preset '{name}'"))
    }

    /// Writes one preset (re-reading the file first), keeping jq's formatting.
    pub fn save(&self, name: &str, preset: &Preset) -> Result<()> {
        let mut all = self.load_all().unwrap_or_default();
        all.insert(name.to_string(), Value::Object(preset.to_json()));
        crate::model::store::write_json_file(
            &self.path(),
            &crate::model::store::data_file("presets", all),
        )
    }

    /// Runs font-switch to make a saved preset the system fonts, at the
    /// theme's sizes (`FONT_SIZES`; without it font-switch would use the
    /// active theme's, which may be another theme).
    pub fn apply(&self, name: &str, sizes: [u32; 3]) -> Result<()> {
        self.run_switch(name, sizes, false)
    }

    /// Applies without restarting Ghostty and i3 (a theme's palette, applied
    /// next, restarts them once for both).
    pub fn apply_quietly(&self, name: &str, sizes: [u32; 3]) -> Result<()> {
        self.run_switch(name, sizes, true)
    }

    fn run_switch(&self, name: &str, sizes: [u32; 3], quiet: bool) -> Result<()> {
        let mut cmd = crate::model::paths::switch_command("font-switch", &self.dir, &self.share);
        cmd.env("FONT_SIZES", sizes.map(|s| s.to_string()).join(" "));
        if quiet {
            cmd.env("NO_RESTART", "1");
        }
        let out = cmd
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .context("running font-switch.sh")?;
        if !out.status.success() {
            bail!(
                "font-switch.sh: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    /// The preset font-switch.sh last applied.
    pub fn active_name() -> Option<String> {
        crate::model::store::read_name_file(&crate::model::paths::fonts_dir().join("current"))
    }

    /// This machine's size offsets (~/.config/host/fonts.json, either key
    /// names); Icons falls back to the Proportional offset, like
    /// font-switch.sh.
    pub fn offsets() -> [i32; 3] {
        let obj: Map<String, Value> =
            std::fs::read_to_string(crate::model::paths::host_fonts_file())
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
        let get = |r: usize| {
            role_entry(&obj, r)
                .and_then(Value::as_i64)
                .and_then(|n| i32::try_from(n).ok())
        };
        let ui = get(1).unwrap_or(0);
        [get(0).unwrap_or(0), ui, get(2).unwrap_or(ui)]
    }
}

/// The active preset from the store: its name, its fonts, whether it is
/// new (not saved yet), and why it could not be loaded, if so. Without an
/// active preset, or when it fails to load, the default one stands in.
pub fn load_active(
    store: &FontStore,
    active: Option<&str>,
) -> (String, Preset, bool, Option<String>) {
    match active.map(|n| (n, store.load(n))) {
        Some((n, Ok(p))) => (n.to_string(), p, false, None),
        Some((n, Err(e))) => (
            n.to_string(),
            Preset::default_preset(),
            true,
            Some(format!("{e:#}")),
        ),
        None => ("untitled".to_string(), Preset::default_preset(), true, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::store::to_jq_string;

    #[test]
    fn presets_round_trip_with_new_keys_and_icons_falls_back() {
        let obj: Map<String, Value> = serde_json::from_str(
            r#"{"mono": {"family": "A", "size": 14}, "ui": {"family": "B", "size": 9}, "note": 1}"#,
        )
        .unwrap();
        let p = Preset::from_json(&obj).unwrap();
        assert_eq!(
            p.fonts[2],
            Font {
                family: "B".into(),
                size: 9
            }
        );
        let out = p.to_json();
        assert_eq!(
            out.keys().collect::<Vec<_>>(),
            ["monospace", "proportional", "icons", "note"]
        );
        // written without sizes (they're the theme's): the same fonts
        assert!(out.values().all(|v| v.get("size").is_none()));
        assert!(Preset::from_json(&out).unwrap().same_fonts(&p));
    }

    #[test]
    fn real_fonts_json_round_trips_byte_for_byte() {
        let dir = crate::model::store::test_data_dir();
        let Ok(text) = std::fs::read_to_string(dir.join("fonts.json")) else {
            eprintln!("skipped: no fonts.json");
            return;
        };
        // any version: rebuilding every preset gives what the migration
        // writes (the file itself, once it's version 3)
        let v: Value = serde_json::from_str(&text).unwrap();
        let current = v["version"] == crate::model::store::FONTS_VERSION;
        let all = crate::model::store::data_map(v, "presets").unwrap();
        let mut rebuilt = Map::new();
        let mut migrated = Map::new();
        for (name, v) in &all {
            let o = v.as_object().unwrap();
            let p = Preset::from_json(o).unwrap();
            rebuilt.insert(name.clone(), Value::Object(p.to_json()));
            migrated.insert(
                name.clone(),
                Value::Object(drop_sizes(rename_roles(o.clone()))),
            );
        }
        let rebuilt = to_jq_string(&crate::model::store::data_file("presets", rebuilt));
        assert_eq!(
            rebuilt,
            to_jq_string(&crate::model::store::data_file("presets", migrated))
        );
        if current {
            assert_eq!(rebuilt, text);
        }
    }

    #[test]
    // Long function (reason verified, phase 6): one scenario in order (write version 2 files, migrate, check, migrate again); each step needs the files of the step before.
    fn version_2_migrates_presets_and_host_offsets_once() {
        let dir =
            std::env::temp_dir().join(format!("theme-studio-fonts-v3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("hosts/laptop")).unwrap();
        std::fs::write(
            dir.join("fonts.json"),
            r#"{"version": 2, "presets": {"A": {"mono": {"family": "M", "size": 14}, "ui": {"family": "U", "size": 10}, "bar": {"family": "B", "size": 9}}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("hosts/laptop/fonts.json"),
            r#"{"mono": 0, "ui": 3, "bar": 3}"#,
        )
        .unwrap();
        let store = FontStore {
            dir: dir.clone(),
            share: dir.clone(),
        };
        let before = store.load("A").unwrap();
        assert_eq!(store.migrate().unwrap(), Some(4));
        assert_eq!(store.migrate().unwrap(), None, "only once");
        assert!(store.load("A").unwrap().same_fonts(&before));
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("fonts.json")).unwrap())
                .unwrap();
        assert_eq!(v["version"], 4);
        assert_eq!(v["presets"]["A"]["monospace"], json!({"family": "M"}));
        assert_eq!(
            v["presets"]["A"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>(),
            ["monospace", "proportional", "icons"]
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("hosts/laptop/fonts.json")).unwrap(),
            "{\n  \"monospace\": 0,\n  \"proportional\": 3,\n  \"icons\": 3\n}\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
