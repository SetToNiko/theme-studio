//! Groups installed font families into a two-level tree for the font list:
//! design (`JetBrains Mono`, Noto, `TeX Gyre`) → variant (Nerd Font, Sans CJK)
//! → families. Groups with a single member are flattened into their parent.

use std::collections::BTreeMap;

use crate::fonts::types::Installed;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Group {
        name: String,
        children: Vec<Node>,
    },
    /// Index into the installed fonts.
    Leaf(usize),
}

impl Node {
    /// Every font under this node.
    pub fn leaves(&self) -> Vec<usize> {
        match self {
            Node::Leaf(i) => vec![*i],
            Node::Group { children, .. } => children.iter().flat_map(Node::leaves).collect(),
        }
    }
}

/// Region codes on CJK font names: "Noto Sans CJK JP" → variant "Sans CJK".
const REGIONS: [&str; 7] = ["HK", "JP", "KR", "SC", "TC", "CN", "TW"];

/// `JetBrainsMono` → `JetBrains Mono`, `GeistMono` → `Geist Mono`: splits
/// off the last capitalised word (only used for Nerd Font names, whose base
/// names are written without spaces).
fn split_camel(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    match (1..chars.len())
        .rev()
        .find(|&i| chars[i].is_uppercase() && chars[i - 1].is_lowercase())
    {
        Some(i) => format!(
            "{} {}",
            chars[..i].iter().collect::<String>(),
            chars[i..].iter().collect::<String>()
        ),
        None => s.to_string(),
    }
}

/// (design tokens, variant tokens) for a family name, before designs are
/// merged across fonts.
fn classify(name: &str) -> (Vec<String>, Vec<String>, bool) {
    if let Some(pos) = name.find(" Nerd Font") {
        let mut base = &name[..pos];
        let nl = base.len() > 2 && base.ends_with("NL");
        if nl {
            base = &base[..base.len() - 2];
        }
        let variant = if nl { "NL Nerd Font" } else { "Nerd Font" };
        return (vec![split_camel(base)], vec![variant.to_string()], true);
    }
    let tokens: Vec<String> = name
        .split([' ', '-'])
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    let (first, rest) = tokens
        .split_first()
        .map(|(f, r)| (f.clone(), r.to_vec()))
        .unwrap_or_default();
    (vec![first], rest, false)
}

/// A design while grouping: its name, its fonts with their variant words,
/// and whether it is a Nerd Font design.
type Design = (String, Vec<(usize, Vec<String>)>, bool);

/// Groups the installed fonts into the tree: design → variant → font.
pub fn build(fonts: &[Installed]) -> Vec<Node> {
    let mut top: Vec<(String, Node)> = Vec::new();
    for (_, (name, members, nerd)) in group_by_design(fonts) {
        let key = name.to_lowercase();
        top.push((key, design_node(name, members, nerd, fonts)));
    }
    top.sort_by(|a, b| a.0.cmp(&b.0));
    top.into_iter().map(|(_, n)| n).collect()
}

/// Puts each font under its design (the key is the lower-case design name).
fn group_by_design(fonts: &[Installed]) -> BTreeMap<String, Design> {
    let mut designs: BTreeMap<String, Design> = BTreeMap::new();
    for (i, f) in fonts.iter().enumerate() {
        let (design, variant, nerd) = classify(&f.family);
        let name = design.join(" ");
        designs
            .entry(name.to_lowercase())
            .or_insert_with(|| (name.clone(), Vec::new(), nerd))
            .1
            .push((i, variant));
    }
    designs
}

/// The node of one design: a group of its variants (a single font or a
/// single variant group is not wrapped again).
fn design_node(
    mut name: String,
    mut members: Vec<(usize, Vec<String>)>,
    nerd: bool,
    fonts: &[Installed],
) -> Node {
    if !nerd {
        move_shared_words_to_name(&mut name, &mut members);
    }
    let mut children = variant_nodes(members, nerd, fonts);
    // A design with a single variant group: its fonts sit directly under it.
    if children.len() == 1
        && let Node::Group {
            children: inner, ..
        } = children[0].clone()
    {
        children = inner;
    }
    if children.len() == 1 {
        children.remove(0)
    } else {
        Node::Group { name, children }
    }
}

/// Longer shared names: every "TeX Gyre X" shares "Gyre", so the design is
/// "TeX Gyre" and the variants are Adventor, Bonum, …
fn move_shared_words_to_name(name: &mut String, members: &mut [(usize, Vec<String>)]) {
    while members.len() > 1
        && members.iter().all(|(_, v)| v.len() > 1)
        && members.iter().all(|(_, v)| v[0] == members[0].1[0])
    {
        *name = format!("{name} {}", members[0].1[0]);
        for (_, v) in &mut *members {
            v.remove(0);
        }
    }
}

/// Puts the fonts of a design under their variant names (region codes
/// dropped, except for Nerd Fonts). The key is the lower-case variant name.
fn group_by_variant(
    members: Vec<(usize, Vec<String>)>,
    nerd: bool,
) -> BTreeMap<String, (String, Vec<usize>)> {
    let mut variants: BTreeMap<String, (String, Vec<usize>)> = BTreeMap::new();
    for (i, mut v) in members {
        if !nerd && v.last().is_some_and(|t| REGIONS.contains(&t.as_str())) {
            v.pop();
        }
        let vname = v.join(" ");
        variants
            .entry(vname.to_lowercase())
            .or_insert_with(|| (vname, Vec::new()))
            .1
            .push(i);
    }
    variants
}

/// The variant groups of a design, sorted by name. A variant with several
/// fonts is a group; a single font stays a leaf.
fn variant_nodes(members: Vec<(usize, Vec<String>)>, nerd: bool, fonts: &[Installed]) -> Vec<Node> {
    let mut children: Vec<(String, Node)> = Vec::new();
    for (_, (vname, leaves)) in group_by_variant(members, nerd) {
        if leaves.len() > 1 && !vname.is_empty() {
            let mut kids: Vec<Node> = leaves.into_iter().map(Node::Leaf).collect();
            kids.sort_by_key(|n| leaf_name(n, fonts));
            children.push((
                vname.to_lowercase(),
                Node::Group {
                    name: vname,
                    children: kids,
                },
            ));
        } else {
            for i in leaves {
                children.push((fonts[i].family.to_lowercase(), Node::Leaf(i)));
            }
        }
    }
    children.sort_by(|a, b| a.0.cmp(&b.0));
    children.into_iter().map(|(_, n)| n).collect()
}

fn leaf_name(n: &Node, fonts: &[Installed]) -> String {
    match n {
        Node::Leaf(i) => fonts[*i].family.to_lowercase(),
        Node::Group { name, .. } => name.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(names: &[&str]) -> Vec<Installed> {
        names
            .iter()
            .map(|n| Installed {
                family: n.to_string(),
                mono: false,
                nerd: false,
                bitmap: false,
                special: false,
            })
            .collect()
    }

    fn shape(nodes: &[Node], fonts: &[Installed]) -> Vec<String> {
        fn walk(n: &Node, depth: usize, fonts: &[Installed], out: &mut Vec<String>) {
            match n {
                Node::Leaf(i) => out.push(format!("{}{}", "  ".repeat(depth), fonts[*i].family)),
                Node::Group { name, children } => {
                    out.push(format!("{}[{name}]", "  ".repeat(depth)));
                    for c in children {
                        walk(c, depth + 1, fonts, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        for n in nodes {
            walk(n, 0, fonts, &mut out);
        }
        out
    }

    /// The fonts on this machine, grouped as agreed: design → variant → font.
    #[test]
    // Long function (reason verified, phase 6): one table of real font names and the tree expected for them; each line stands alone, so splitting it only hides the table.
    fn groups_real_font_names() {
        let fonts = installed(&[
            "DejaVu Sans",
            "DejaVu Sans Mono",
            "DejaVu Serif",
            "DejaVu Math TeX Gyre",
            "FreeMono",
            "FreeSans",
            "cursor.pcf",
            "GeistMono Nerd Font",
            "GeistMono Nerd Font Mono",
            "GeistMono Nerd Font Propo",
            "JetBrainsMono Nerd Font",
            "JetBrainsMono Nerd Font Mono",
            "JetBrainsMono Nerd Font Propo",
            "JetBrainsMonoNL Nerd Font",
            "JetBrainsMonoNL Nerd Font Mono",
            "JetBrainsMonoNL Nerd Font Propo",
            "Noto Color Emoji",
            "Noto Sans CJK HK",
            "Noto Sans CJK JP",
            "Noto Serif CJK HK",
            "Noto Serif CJK JP",
            "Noto Sans Mono CJK HK",
            "Noto Sans Mono CJK JP",
            "TeX Gyre Adventor",
            "TeX Gyre Heros",
            "TeX Gyre Heros Cn",
            "Unifont",
            "Unifont-JP",
            "Unifont Upper",
        ]);
        let tree = build(&fonts);
        let s = shape(&tree, &fonts);
        let expect = [
            "cursor.pcf",
            "[DejaVu]",
            "  DejaVu Math TeX Gyre",
            "  DejaVu Sans",
            "  DejaVu Sans Mono",
            "  DejaVu Serif",
            "FreeMono",
            "FreeSans",
            "[Geist Mono]",
            "  GeistMono Nerd Font",
            "  GeistMono Nerd Font Mono",
            "  GeistMono Nerd Font Propo",
            "[JetBrains Mono]",
            "  [Nerd Font]",
            "    JetBrainsMono Nerd Font",
            "    JetBrainsMono Nerd Font Mono",
            "    JetBrainsMono Nerd Font Propo",
            "  [NL Nerd Font]",
            "    JetBrainsMonoNL Nerd Font",
            "    JetBrainsMonoNL Nerd Font Mono",
            "    JetBrainsMonoNL Nerd Font Propo",
            "[Noto]",
            "  Noto Color Emoji",
            "  [Sans CJK]",
            "    Noto Sans CJK HK",
            "    Noto Sans CJK JP",
            "  [Sans Mono CJK]",
            "    Noto Sans Mono CJK HK",
            "    Noto Sans Mono CJK JP",
            "  [Serif CJK]",
            "    Noto Serif CJK HK",
            "    Noto Serif CJK JP",
            "[TeX Gyre]",
            "  TeX Gyre Adventor",
            "  TeX Gyre Heros",
            "  TeX Gyre Heros Cn",
            "[Unifont]",
            "  Unifont",
            "  Unifont Upper",
            "  Unifont-JP",
        ];
        assert_eq!(s, expect, "\n{}", s.join("\n"));
        let all: usize = tree.iter().map(|n| n.leaves().len()).sum();
        assert_eq!(all, fonts.len(), "every font appears once");
    }
}
