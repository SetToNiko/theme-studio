//! The font lists: Fontsource's catalogue (Google and other fonts) and the
//! Nerd Fonts release's assets, fetched in a background thread.

use std::sync::mpsc::Sender;

use serde_json::Value;

use super::net::cached;
use super::{Catalogue, Entry, Msg, Source};

/// Fontsource's font list.
pub fn parse_fontsource(v: &Value) -> Vec<Entry> {
    let Some(list) = v.as_array() else {
        return Vec::new();
    };
    list.iter().filter_map(fontsource_entry).collect()
}

/// One font of Fontsource's list (none without an `id`).
fn fontsource_entry(f: &Value) -> Option<Entry> {
    let s = |k: &str| f.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Some(Entry {
        id: f.get("id")?.as_str()?.to_string(),
        family: s("family"),
        category: s("category"),
        license: s("license"),
        weights: f
            .get("weights")
            .and_then(Value::as_array)
            .map_or(Vec::new(), |w| w.iter().filter_map(Value::as_u64).collect()),
        italic: f
            .get("styles")
            .and_then(Value::as_array)
            .is_some_and(|s| s.iter().any(|x| x == "italic")),
        variable: f
            .get("variable")
            .is_some_and(|x| x.as_bool() == Some(true) || x.is_object()),
        source: if s("type") == "google" {
            Source::Google
        } else {
            Source::Fontsource
        },
        subset: Some(s("defSubset"))
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| "latin".into()),
    })
}

/// The Nerd Fonts release's `.tar.xz` assets.
pub fn parse_nerd(v: &Value) -> (String, Vec<Entry>) {
    let tag = v
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let entries = v
        .get("assets")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(nerd_entry).collect())
        .unwrap_or_default();
    (tag, entries)
}

/// One asset of the release (none unless it is a `.tar.xz` with a URL).
fn nerd_entry(x: &Value) -> Option<Entry> {
    let name = x
        .get("name")?
        .as_str()?
        .strip_suffix(".tar.xz")?
        .to_string();
    Some(Entry {
        id: format!("nerd-{}", name.to_lowercase()),
        family: format!("{name} Nerd Font"),
        category: "nerd font".into(),
        license: String::new(),
        weights: Vec::new(),
        italic: false,
        variable: false,
        source: Source::Nerd {
            url: x.get("browser_download_url")?.as_str()?.to_string(),
            size: x.get("size").and_then(Value::as_u64).unwrap_or(0),
            asset: name,
        },
        subset: String::new(),
    })
}

/// Fetches both catalogues in the background.
pub fn load_catalogue(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let google = cached("fontsource.json", "https://api.fontsource.org/v1/fonts")
            .map(|v| parse_fontsource(&v));
        let nerd = cached(
            "nerd-fonts.json",
            "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest",
        )
        .map(|v| parse_nerd(&v));
        let result = match (google, nerd) {
            (Err(e), _) => Err(format!("{e:#}")),
            (Ok(google), nerd) => {
                let (nerd_tag, nerd) = nerd.unwrap_or_default();
                Ok(Catalogue {
                    google,
                    nerd,
                    nerd_tag,
                })
            }
        };
        let _ = tx.send(Msg::Catalogue(result));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalogues_parse() {
        let fs = json!([
            {"id": "roboto-mono", "family": "Roboto Mono", "category": "monospace", "license": "Apache-2.0",
             "weights": [100, 400, 700], "styles": ["normal", "italic"], "variable": true, "type": "google"},
            {"id": "zen-loop", "family": "Zen Loop", "category": "display", "license": "OFL-1.1",
             "weights": [400], "styles": ["normal"], "variable": false, "type": "other"}
        ]);
        let e = parse_fontsource(&fs);
        assert_eq!(e.len(), 2);
        assert_eq!(
            (e[0].family.as_str(), e[0].italic, e[0].variable),
            ("Roboto Mono", true, true)
        );
        assert_eq!(e[1].source, Source::Fontsource);
        let nerd = json!({"tag_name": "v3.5.1", "assets": [
            {"name": "JetBrainsMono.tar.xz", "size": 10, "browser_download_url": "https://x/JetBrainsMono.tar.xz"},
            {"name": "JetBrainsMono.zip", "size": 12, "browser_download_url": "https://x/JetBrainsMono.zip"}
        ]});
        let (tag, n) = parse_nerd(&nerd);
        assert_eq!((tag.as_str(), n.len()), ("v3.5.1", 1));
        assert_eq!(n[0].family, "JetBrainsMono Nerd Font");
        assert_eq!(n[0].id, "nerd-jetbrainsmono");
    }
}
