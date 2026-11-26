//! Drawing the Settings page (F3): tab strip, then the Keymaps, Behaviour,
//! History or README tab. Everything clickable goes into the hit map with a
//! hover look.
//!
//! - `keymaps.rs`: the Keymaps tab.
//! - `behaviour.rs`: the Behaviour tab.
//! - `history.rs`: the History tab.
//! - `readme.rs`: the README tab.
//!
//! This file keeps the tab strip, the Save and Discard buttons and the small
//! helpers that the tabs share.

mod behaviour;
mod history;
mod keymaps;
mod readme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::app::state::hit::{Target, hover, hover_line, hover_row};
use crate::model::color::Rgb;
use crate::settings_page::{TABS, Tab};

/// Keeps `sel` on screen in a list `height` rows tall.
fn follow(scroll: &mut usize, sel: usize, height: usize) {
    if height == 0 {
        return;
    }
    if sel < *scroll {
        *scroll = sel;
    } else if sel >= *scroll + height {
        *scroll = sel + 1 - height;
    }
}

fn row_rect(area: Rect, i: usize) -> Rect {
    Rect {
        y: area.y + i as u16,
        height: 1,
        ..area
    }
}

fn selected_style(on: bool, focused: bool) -> Style {
    match (on, focused) {
        (true, true) => Style::new().reversed(),
        (true, false) => Style::new().bold().underlined(),
        _ => Style::new(),
    }
}

fn colour_span(c: Rgb, text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::new().fg(Color::Rgb(c.0, c.1, c.2)))
}

/// A value as text, with a swatch in front when it's a colour.
fn value_spans(v: &str, style: Style) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    if v.len() == 7
        && let Some(c) = Rgb::parse(v)
    {
        out.push(colour_span(c, "██ "));
    }
    out.push(Span::styled(v.to_string(), style));
    out
}

// Long function: frame, the two tab-strip positions and the tab strip are laid out in one top-to-bottom pass.
pub fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::bordered()
        .title(" Settings ")
        .border_style(Style::new().yellow());
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    // the page's tabs: on top or at the bottom (a setting)
    let (tabs, body) = if app.settings.layout.inner_tabs_on_top() {
        let [t, _, b] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        (t, b)
    } else {
        let [b, _, t] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        (t, b)
    };
    let tabs_focused = app.settings_page.tabs_focused && !app.layout.page_tabs_focused;
    // tab strip
    let mut x = tabs.x + 1;
    for (i, (tab, name)) in TABS.iter().enumerate() {
        let text = format!(" {name} ");
        let w = text.chars().count() as u16;
        let r = Rect {
            x,
            y: tabs.y,
            width: w.min(tabs.right().saturating_sub(x)),
            height: 1,
        };
        // with the keyboard on the tabs, the current one is yellow
        let mut style = match (*tab == app.settings_page.tab, tabs_focused) {
            (true, true) => Style::new().black().on_yellow().bold(),
            (true, false) => Style::new().reversed().bold(),
            _ => Style::new().dark_gray(),
        };
        let mut line = Line::from(Span::styled(text, style));
        if app.hits.add(r, Target::SettingsTab(i as u8)) {
            style = hover_row(style);
            line = hover_line(line, 0);
        }
        f.render_widget(Paragraph::new(line).style(style), r);
        x += w + 1;
    }
    match app.settings_page.tab {
        Tab::Keymaps => keymaps::draw_keymaps(f, app, body),
        Tab::Behaviour => behaviour::draw_behaviour(f, app, body),
        Tab::History => history::draw_history(f, app, body),
        Tab::Readme => readme::draw_readme(f, app, body),
    }
}

/// Cuts `s` to `n` characters, ending with `…` when it was longer.
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// The Save and Discard buttons on the frame's bottom edge, while there are
/// unsaved changes; drawn with the live keymap (keys change only on Save).
// Long function: lays out the note and two buttons in one row, left to right.
pub fn draw_save_buttons(f: &mut Frame, app: &mut App, area: Rect, changes: usize) {
    if changes == 0 || area.height < 2 {
        return;
    }
    use crate::keymap::Context;
    use crate::keymap::action::Action;
    let key = |a: Action| {
        app.keymap
            .key_for(&[Context::Settings, Context::Global], a)
            .map(|k| crate::ui::widgets::friendly(&k))
            .unwrap_or_default()
    };
    let note = format!(" ● {changes} unsaved ");
    let save = format!(" {} Save ", key(Action::Save));
    let discard = format!(" {} Discard ", key(Action::Discard));
    let w = |s: &str| s.chars().count() as u16;
    let total = w(&note) + w(&save) + w(&discard) + 2;
    let y = area.bottom() - 1;
    let mut x = area.right().saturating_sub(total + 2);
    f.render_widget(
        Paragraph::new(Span::styled(note.clone(), Style::new().yellow())),
        Rect {
            x,
            y,
            width: w(&note),
            height: 1,
        },
    );
    x += w(&note) + 1;
    for (text, id, base) in [
        (save, 's', Style::new().black().on_yellow().bold()),
        (discard, 'd', Style::new().reversed()),
    ] {
        let r = Rect {
            x,
            y,
            width: w(&text),
            height: 1,
        };
        let style = if app.hits.add(r, Target::SettingsButton(id)) {
            hover(base)
        } else {
            base
        };
        f.render_widget(Paragraph::new(Span::styled(text.clone(), style)), r);
        x += w(&text) + 1;
    }
}
