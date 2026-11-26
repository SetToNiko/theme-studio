//! A key press in Vim notation: parsing (`<C-r>`, `<leader>sp`) and display.
//!
//! [`Key`] is one normalised key press. [`display_seq`] shows a whole
//! sequence of keys for the help screen and the leader popup.

use anyhow::{Result, anyhow, bail};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// One key press, normalised: Shift is part of the character for letters
/// and symbols (`J`, `?`), and Shift+Tab is `<S-Tab>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl Key {
    pub const fn new(code: KeyCode, mods: KeyModifiers) -> Key {
        Key { code, mods }
    }

    pub fn from_event(k: KeyEvent) -> Key {
        let mut mods =
            k.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT);
        let mut code = k.code;
        match code {
            // the kitty keyboard protocol sends Ctrl+Shift+S as s + Shift
            KeyCode::Char(c) => {
                if mods.contains(KeyModifiers::SHIFT) {
                    code = KeyCode::Char(c.to_ascii_uppercase());
                }
                mods.remove(KeyModifiers::SHIFT);
            }
            KeyCode::BackTab => mods.remove(KeyModifiers::SHIFT),
            _ => {}
        }
        Key { code, mods }
    }

    /// Parses one key sequence, e.g. `<leader>cp`, `gg`, `<C-x><C-s>`.
    ///
    /// # Errors
    /// Fails when `s` is empty, or a `<...>` key name is not known (see `Key::parse_named`).
    pub fn parse_seq(s: &str, leader: Key) -> Result<Vec<Key>> {
        let mut out = Vec::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '<' && s.contains('>') {
                let mut name = String::new();
                for c in chars.by_ref() {
                    if c == '>' {
                        break;
                    }
                    name.push(c);
                }
                out.push(Key::parse_named(&name, leader)?);
            } else {
                out.push(Key::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        }
        if out.is_empty() {
            bail!("empty key sequence");
        }
        Ok(out)
    }

    /// `leader`, `space`, `CR`, `C-r`, `S-Tab`, `M-x`, `F5`, `Up`, ….
    ///
    /// # Errors
    /// Fails when `name` is not a known key name.
    fn parse_named(name: &str, leader: Key) -> Result<Key> {
        let lower = name.to_lowercase();
        if lower == "leader" {
            return Ok(leader);
        }
        let mut mods = KeyModifiers::NONE;
        let mut rest = name;
        loop {
            let l = rest.to_lowercase();
            if l.len() > 2 && l.starts_with("c-") {
                mods |= KeyModifiers::CONTROL;
            } else if l.len() > 2 && (l.starts_with("m-") || l.starts_with("a-")) {
                mods |= KeyModifiers::ALT;
            } else if l.len() > 2 && l.starts_with("s-") {
                mods |= KeyModifiers::SHIFT;
            } else {
                break;
            }
            rest = &rest[2..];
        }
        let code = match rest.to_lowercase().as_str() {
            "space" => KeyCode::Char(' '),
            "cr" | "enter" | "return" => KeyCode::Enter,
            "esc" => KeyCode::Esc,
            "tab" if mods.contains(KeyModifiers::SHIFT) => {
                mods.remove(KeyModifiers::SHIFT);
                KeyCode::BackTab
            }
            "tab" => KeyCode::Tab,
            "bs" | "backspace" => KeyCode::Backspace,
            "del" | "delete" => KeyCode::Delete,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "lt" => KeyCode::Char('<'),
            f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<u8>().is_ok() => {
                KeyCode::F(f[1..].parse().unwrap())
            }
            _ if rest.chars().count() == 1 => {
                let c = rest.chars().next().unwrap();
                // Shift on a character is its capital, as key events report it:
                // <C-S-z> and <C-Z> are both Ctrl+Shift+Z
                if mods.contains(KeyModifiers::SHIFT) {
                    mods.remove(KeyModifiers::SHIFT);
                    return Ok(Key {
                        code: KeyCode::Char(c.to_ascii_uppercase()),
                        mods,
                    });
                }
                KeyCode::Char(c)
            }
            _ => return Err(anyhow!("unknown key <{name}>")),
        };
        Ok(Key { code, mods })
    }

    /// Vim notation, e.g. `j`, `<C-r>`, `<S-Tab>`, `<Space>`.
    pub fn display(&self) -> String {
        let mut prefix = String::new();
        if self.mods.contains(KeyModifiers::CONTROL) {
            prefix.push_str("C-");
        }
        if self.mods.contains(KeyModifiers::ALT) {
            prefix.push_str("M-");
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            prefix.push_str("S-");
        }
        let name = match self.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char('<') => "lt".to_string(),
            KeyCode::Char(c) if prefix.is_empty() => return c.to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "CR".into(),
            KeyCode::Esc => "Esc".into(),
            KeyCode::Tab => "Tab".into(),
            KeyCode::BackTab => "S-Tab".into(),
            KeyCode::Backspace => "BS".into(),
            KeyCode::Delete => "Del".into(),
            KeyCode::Up => "Up".into(),
            KeyCode::Down => "Down".into(),
            KeyCode::Left => "Left".into(),
            KeyCode::Right => "Right".into(),
            KeyCode::Home => "Home".into(),
            KeyCode::End => "End".into(),
            KeyCode::PageUp => "PageUp".into(),
            KeyCode::PageDown => "PageDown".into(),
            KeyCode::F(n) => format!("F{n}"),
            other => format!("{other:?}"),
        };
        format!("<{prefix}{name}>")
    }
}

/// Display a sequence; a leading leader key is shown as `<leader>` (later
/// in a sequence it's just that key: Emacs's C-x C-c, with C-c the leader).
pub fn display_seq(keys: &[Key], leader: Key) -> String {
    keys.iter()
        .enumerate()
        .map(|(i, k)| {
            if i == 0 && *k == leader {
                "<leader>".to_string()
            } else {
                k.display()
            }
        })
        .collect()
}
