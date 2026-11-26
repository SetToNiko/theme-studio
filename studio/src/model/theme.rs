//! Themes: a name for a palette, a font preset and the fonts' sizes, saved
//! as one in themes.json (`{"version": 2, "themes": {name: {palette, fonts,
//! sizes: {monospace, proportional, icons}}}}`). The palette and preset stay
//! in palettes.json and fonts.json, so they can be shared; the sizes are the
//! theme's own, so swapping the preset keeps them. The active theme is
//! recorded in ~/.config/colors/theme.

use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::fonts::types::ROLES;

/// themes.json's version. 2: each theme has its fonts' sizes.
pub const THEMES_VERSION: u64 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub palette: String,
    pub fonts: String,
    /// Base sizes for Monospace, Proportional and Icons (before the host
    /// offsets). None in a version 1 file, until it's migrated.
    pub sizes: Option<[u32; 3]>,
}

impl Theme {
    fn from_json(t: &Value) -> Option<Theme> {
        let sizes = t.get("sizes").and_then(|s| {
            let get = |r: usize| s.get(ROLES[r])?.as_u64().map(|n| n as u32);
            Some([get(0)?, get(1)?, get(2)?])
        });
        Some(Theme {
            palette: t.get("palette")?.as_str()?.to_string(),
            fonts: t.get("fonts")?.as_str()?.to_string(),
            sizes,
        })
    }

    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("palette".into(), json!(self.palette));
        m.insert("fonts".into(), json!(self.fonts));
        if let Some(s) = self.sizes {
            let mut sizes = Map::new();
            for (r, n) in ROLES.iter().zip(s) {
                sizes.insert(r.to_string(), json!(n));
            }
            m.insert("sizes".into(), Value::Object(sizes));
        }
        Value::Object(m)
    }

    /// The same palette and font preset (sizes aside).
    pub(crate) fn same_parts(&self, other: &Theme) -> bool {
        self.palette == other.palette && self.fonts == other.fonts
    }
}

pub struct ThemeStore {
    /// The folder with palettes.json (themes.json sits next to it).
    pub dir: PathBuf,
}

impl ThemeStore {
    pub fn path(&self) -> PathBuf {
        self.dir.join("themes.json")
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    /// Every theme, in file order.
    pub fn load_all(&self) -> Vec<(String, Theme)> {
        let v: Value = std::fs::read_to_string(self.path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        v.get("themes")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, t)| Some((k.clone(), Theme::from_json(t)?)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Version 1 themes get sizes: their font preset's (fonts.json before
    /// version 4 has them), else the defaults. Run before fonts.json's
    /// migration drops them from the presets. The version it now has, if it
    /// did.
    pub fn migrate(&self, fonts: &crate::fonts::presets::FontStore) -> Result<Option<u64>> {
        let Ok(text) = std::fs::read_to_string(self.path()) else {
            return Ok(None);
        };
        let v: Value = serde_json::from_str(&text)?;
        if v.get("version").and_then(Value::as_u64).unwrap_or(1) >= THEMES_VERSION {
            return Ok(None);
        }
        let mut m = Map::new();
        for (n, mut t) in self.load_all() {
            if t.sizes.is_none() {
                t.sizes = Some(
                    fonts
                        .load(&t.fonts)
                        .map(|p| p.sizes())
                        .unwrap_or(crate::fonts::presets::DEFAULT_SIZES),
                );
            }
            m.insert(n, t.to_json());
        }
        crate::model::store::write_json_file(
            &self.path(),
            &json!({"version": THEMES_VERSION, "themes": m}),
        )?;
        Ok(Some(THEMES_VERSION))
    }

    pub fn get(&self, name: &str) -> Option<Theme> {
        self.load_all()
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t)
    }

    /// Adds or replaces a theme (in place, keeping the file's order).
    pub fn save(&self, name: &str, t: &Theme) -> Result<()> {
        let mut m = Map::new();
        let mut placed = false;
        for (n, old) in self.load_all() {
            let v = if n == name {
                placed = true;
                t.clone()
            } else {
                old
            };
            m.insert(n, v.to_json());
        }
        if !placed {
            m.insert(name.to_string(), t.to_json());
        }
        crate::model::store::write_json_file(
            &self.path(),
            &json!({"version": THEMES_VERSION, "themes": m}),
        )
    }

    /// The theme applied last (themes-switch.sh or the studio).
    pub fn active_name() -> Option<String> {
        let s = std::fs::read_to_string(crate::model::paths::colors_dir().join("theme")).ok()?;
        Some(s.trim().to_string()).filter(|s| !s.is_empty())
    }

    pub fn set_active(name: &str) -> Result<()> {
        // tests never touch your ~/.config
        if cfg!(test) {
            return Ok(());
        }
        let dir = crate::model::paths::colors_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("theme"), format!("{name}\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_save_in_place_and_read_back() {
        let d = std::env::temp_dir().join(format!("theme-studio-themes-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let s = ThemeStore { dir: d.clone() };
        assert!(s.load_all().is_empty() && !s.exists());
        let t = |p: &str, f: &str| Theme {
            palette: p.into(),
            fonts: f.into(),
            sizes: Some([14, 9, 9]),
        };
        s.save("Work", &t("One", "Mono")).unwrap();
        s.save("Play", &t("Two", "Mono")).unwrap();
        s.save("Work", &t("Two", "Wide")).unwrap();
        let all = s.load_all();
        assert_eq!(
            all.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
            ["Work", "Play"]
        );
        assert_eq!(s.get("Work"), Some(t("Two", "Wide")));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(s.path()).unwrap()).unwrap();
        assert_eq!(v["version"], 2);
        assert_eq!(
            v["themes"]["Work"]["sizes"],
            json!({"monospace": 14, "proportional": 9, "icons": 9})
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn version_1_themes_take_their_presets_sizes_once() {
        let d = std::env::temp_dir().join(format!("theme-studio-themes-v2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("fonts.json"),
            r#"{"version": 3, "presets": {"P": {"monospace": {"family": "M", "size": 13}, "proportional": {"family": "U", "size": 10}, "icons": {"family": "B", "size": 9}}}}"#,
        )
        .unwrap();
        std::fs::write(
            d.join("themes.json"),
            r#"{"version": 1, "themes": {"A": {"palette": "x", "fonts": "P"}, "B": {"palette": "y", "fonts": "gone"}}}"#,
        )
        .unwrap();
        let themes = ThemeStore { dir: d.clone() };
        let fonts = crate::fonts::presets::FontStore {
            dir: d.clone(),
            share: d.clone(),
        };
        // themes first (from the presets' sizes), then fonts.json drops them
        assert_eq!(themes.migrate(&fonts).unwrap(), Some(2));
        assert_eq!(themes.migrate(&fonts).unwrap(), None, "only once");
        assert_eq!(fonts.migrate().unwrap(), Some(4));
        assert_eq!(themes.get("A").unwrap().sizes, Some([13, 10, 9]));
        assert_eq!(
            themes.get("B").unwrap().sizes,
            Some(crate::fonts::presets::DEFAULT_SIZES),
            "a missing preset: the defaults"
        );
        let f: Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("fonts.json")).unwrap()).unwrap();
        assert_eq!(f["version"], 4);
        assert_eq!(f["presets"]["P"]["monospace"], json!({"family": "M"}));
        std::fs::remove_dir_all(d).unwrap();
    }
}
