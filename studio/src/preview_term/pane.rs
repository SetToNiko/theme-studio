//! One program in a hidden terminal: the pane, its colours, and the key bytes.
//!
//! A [`Pane`] runs a real program (nvim, a shell, yazi) in a pseudo-terminal and
//! keeps the screen it draws. [`TermColors`] maps the screen's colours to the draft
//! palette. [`key_bytes`] turns a key press into the bytes a terminal sends.

use std::io::{Read, Write};
use std::num::NonZeroU16;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::core::color::Rgb;
use crate::core::palette::{self, Palette};
use crate::core::store::TerminalMap;

/// A program running in a hidden terminal.
pub struct Pane {
    parser: Arc<Mutex<vt100::Parser>>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    size: (u16, u16),
    /// Typed once the program has drawn something (the shell's first prompt),
    /// so it isn't echoed early as type-ahead.
    pub(super) pending_input: Option<Vec<u8>>,
    spawned: std::time::Instant,
}

fn nz(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n.max(1)).unwrap()
}

impl Pane {
    pub(super) fn spawn(
        cmd: CommandBuilder,
        rows: u16,
        cols: u16,
        dirty: Arc<AtomicBool>,
    ) -> Result<Pane> {
        let (rows, cols) = (rows.max(2), cols.max(2));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("opening a pty")?;
        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let parser = Arc::new(Mutex::new(vt100::Parser::new(nz(rows), nz(cols), 0)));
        let p = parser.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16384];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        p.lock().unwrap().process(&buf[..n]);
                        dirty.store(true, Ordering::Relaxed);
                    }
                }
            }
        });
        Ok(Pane {
            parser,
            master: pair.master,
            writer,
            child,
            size: (rows, cols),
            pending_input: None,
            spawned: std::time::Instant::now(),
        })
    }

    /// Follows the preview window's size, like a real terminal window would.
    pub(super) fn resize(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = (rows.max(2), cols.max(2));
        if self.size != (rows, cols) {
            self.size = (rows, cols);
            let _ = self.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
            self.parser.lock().unwrap().set_size(nz(rows), nz(cols));
        }
    }

    /// Sends `pending_input` once the program has printed something.
    pub(crate) fn flush_pending(&mut self) {
        if self.pending_input.is_none() {
            return;
        }
        let ready = !self
            .parser
            .lock()
            .unwrap()
            .screen()
            .contents()
            .trim()
            .is_empty()
            || self.spawned.elapsed() > Duration::from_secs(3);
        if ready && let Some(bytes) = self.pending_input.take() {
            self.write(&bytes);
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self
            .writer
            .write_all(bytes)
            .and_then(|_| self.writer.flush());
    }

    pub(super) fn bracketed_paste(&self) -> bool {
        self.parser.lock().unwrap().screen().bracketed_paste()
    }

    pub(super) fn application_cursor(&self) -> bool {
        self.parser.lock().unwrap().screen().application_cursor()
    }

    /// Draws the terminal screen, mapping its colours through `colors`.
    pub(crate) fn render(&self, buf: &mut Buffer, area: Rect, colors: &TermColors, cursor: bool) {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        for y in 0..area.height.min(rows.get()) {
            for x in 0..area.width.min(cols.get()) {
                let Some(cell) = screen.cell(y, x) else {
                    continue;
                };
                let Some(out) = buf.cell_mut((area.x + x, area.y + y)) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut fg = colors.map(cell.fgcolor(), colors.fg);
                let mut bg = colors.map(cell.bgcolor(), colors.bg);
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if cell.dim() {
                    fg = blend(fg, bg, 0.4);
                }
                let mut style = Style::new().fg(fg.color()).bg(bg.color());
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                let sym = if cell.has_contents() {
                    cell.contents()
                } else {
                    " "
                };
                out.set_symbol(sym).set_style(style);
            }
        }
        if cursor && !screen.hide_cursor() {
            let (cy, cx) = screen.cursor_position();
            if cy < area.height
                && cx < area.width
                && let Some(out) = buf.cell_mut((area.x + cx, area.y + cy))
            {
                out.set_style(Style::new().fg(colors.bg.color()).bg(colors.fg.color()));
            }
        }
    }
}

impl Drop for Pane {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn blend(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let ch = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Rgb(ch(a.0, b.0), ch(a.1, b.1), ch(a.2, b.2))
}

/// Terminal colours as Ghostty would set them from the palette
/// (terminal-colors.json): default fg/bg and the 16 ANSI colours.
pub struct TermColors {
    pub fg: Rgb,
    pub bg: Rgb,
    pub ansi: [Rgb; 16],
}

impl TermColors {
    pub fn new(p: &Palette, map: Option<&TerminalMap>) -> TermColors {
        let k = |name: &str, fallback: &str| p.get(palette::key(name).unwrap_or(fallback));
        let mut ansi = [Rgb(0, 0, 0); 16];
        const DEFAULT: [&str; 16] = [
            "bg1", "red", "green", "yellow", "blue", "purple", "aqua", "fg0", "grey0", "red",
            "green", "yellow", "blue", "purple", "aqua", "fg1",
        ];
        for (i, slot) in ansi.iter_mut().enumerate() {
            let name = map
                .and_then(|m| m.ansi.get(i))
                .map(String::as_str)
                .unwrap_or(DEFAULT[i]);
            *slot = k(name, DEFAULT[i]);
        }
        TermColors {
            fg: k(map.map_or("fg0", |m| m.foreground.as_str()), "fg0"),
            bg: k(map.map_or("bg0", |m| m.background.as_str()), "bg0"),
            ansi,
        }
    }

    fn map(&self, c: vt100::Color, default: Rgb) -> Rgb {
        match c {
            vt100::Color::Default => default,
            vt100::Color::Idx(i) if i < 16 => self.ansi[i as usize],
            vt100::Color::Idx(i) => xterm256(i),
            vt100::Color::Rgb(r, g, b) => Rgb(r, g, b),
        }
    }
}

/// The standard xterm colours 16-255 (6×6×6 cube, then greys).
fn xterm256(i: u8) -> Rgb {
    if i >= 232 {
        let v = 8 + (i - 232) * 10;
        return Rgb(v, v, v);
    }
    let i = i - 16;
    let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
    Rgb(level(i / 36), level(i / 6 % 6), level(i % 6))
}

/// Bytes a terminal sends for a key.
pub fn key_bytes(k: KeyEvent, app_cursor: bool) -> Vec<u8> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    let arrow = |c: char| {
        if app_cursor {
            format!("\x1bO{c}")
        } else {
            format!("\x1b[{c}")
        }
        .into_bytes()
    };
    let mut out = match k.code {
        KeyCode::Char(c) if ctrl && c.is_ascii_alphabetic() => {
            vec![(c.to_ascii_lowercase() as u8) & 0x1f]
        }
        KeyCode::Char(' ') if ctrl => vec![0],
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => arrow('A'),
        KeyCode::Down => arrow('B'),
        KeyCode::Right => arrow('C'),
        KeyCode::Left => arrow('D'),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n @ 1..=4) => format!("\x1bO{}", (b'P' + n - 1) as char).into_bytes(),
        KeyCode::F(n @ 5..=11) => {
            let code = [15, 17, 18, 19, 20, 21, 23][(n - 5) as usize];
            format!("\x1b[{code}~").into_bytes()
        }
        _ => Vec::new(),
    };
    if alt && !out.is_empty() {
        out.insert(0, 0x1b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xterm_colours() {
        assert_eq!(xterm256(16), Rgb(0, 0, 0));
        assert_eq!(xterm256(196), Rgb(255, 0, 0));
        assert_eq!(xterm256(231), Rgb(255, 255, 255));
        assert_eq!(xterm256(232), Rgb(8, 8, 8));
        assert_eq!(xterm256(255), Rgb(238, 238, 238));
    }

    #[test]
    fn keys_become_terminal_bytes() {
        let k = |c, m| KeyEvent::new(c, m);
        assert_eq!(
            key_bytes(k(KeyCode::Char('a'), KeyModifiers::NONE), false),
            b"a"
        );
        assert_eq!(
            key_bytes(k(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            [3]
        );
        assert_eq!(
            key_bytes(k(KeyCode::Up, KeyModifiers::NONE), false),
            b"\x1b[A"
        );
        assert_eq!(
            key_bytes(k(KeyCode::Up, KeyModifiers::NONE), true),
            b"\x1bOA"
        );
        assert_eq!(
            key_bytes(k(KeyCode::Char('x'), KeyModifiers::ALT), false),
            b"\x1bx"
        );
        assert_eq!(
            key_bytes(k(KeyCode::F(5), KeyModifiers::NONE), false),
            b"\x1b[15~"
        );
    }

    /// A real program in a pane: its ANSI colours come out as the palette's.
    #[test]
    fn pane_maps_ansi_colours_to_the_palette() {
        let dirty = Arc::new(AtomicBool::new(false));
        let mut cmd = CommandBuilder::new("printf");
        cmd.arg("\x1b[31mred\x1b[0m \x1b[42mbg\x1b[0m");
        let Ok(pane) = Pane::spawn(cmd, 5, 20, dirty.clone()) else {
            eprintln!("skipped: no pty (build sandbox)");
            return;
        };
        for _ in 0..100 {
            if pane
                .parser
                .lock()
                .unwrap()
                .screen()
                .contents()
                .contains("bg")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(dirty.load(Ordering::Relaxed));
        let p = Palette::blank();
        let colors = TermColors::new(&p, None);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        pane.render(&mut buf, Rect::new(0, 0, 20, 5), &colors, false);
        assert_eq!(buf[(0, 0)].symbol(), "r");
        assert_eq!(buf[(0, 0)].fg, p.get("red").color());
        assert_eq!(buf[(4, 0)].bg, p.get("green").color());
        assert_eq!(
            buf[(10, 0)].bg,
            p.get("bg0").color(),
            "default background is bg0"
        );
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;

    fn wait_for(pane: &Pane, text: &str) -> bool {
        for _ in 0..150 {
            if pane
                .parser
                .lock()
                .unwrap()
                .screen()
                .contents()
                .contains(text)
            {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Keys typed into a pane reach the program; pending input waits for
    /// the program's first output.
    #[test]
    fn typing_reaches_the_program() {
        let dirty = Arc::new(AtomicBool::new(false));
        let mut cmd = CommandBuilder::new("bash");
        cmd.args(["--norc", "--noprofile", "-i"]);
        cmd.env("PS1", "READY> ");
        let Ok(mut pane) = Pane::spawn(cmd, 10, 40, dirty) else {
            eprintln!("skipped: no pty (build sandbox)");
            return;
        };
        pane.pending_input = Some(b"echo $((6*7))\n".to_vec());
        assert!(wait_for(&pane, "READY>"));
        pane.flush_pending();
        assert!(wait_for(&pane, "42"));
        for c in "echo typed".chars() {
            let bytes = key_bytes(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE), false);
            pane.write(&bytes);
        }
        pane.write(&key_bytes(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            false,
        ));
        assert!(wait_for(&pane, "\ntyped"));
        // the demo line was typed after the prompt, so it appears once
        let screen = pane.parser.lock().unwrap().screen().contents();
        assert_eq!(screen.matches("echo $((6*7))").count(), 1, "{screen}");
    }
}
