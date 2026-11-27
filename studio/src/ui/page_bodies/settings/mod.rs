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
use crate::keymap::action::Action;
use crate::keymap::context::Context;
use crate::model::color::Rgb;
use crate::settings_page::state::{TABS, Tab};
use crate::ui::widgets::{to_u8, to_u16};

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

/// Draws the tab strip on `tabs`; the current tab is yellow while the
/// keyboard is on the tabs.
fn draw_tab_strip(f: &mut Frame, app: &mut App, tabs: Rect) {
    let tabs_focused = app.settings_page.tabs_focused && !app.layout.page_tabs_focused;
    let mut x = tabs.x + 1;
    for (index, (tab, name)) in TABS.iter().enumerate() {
        let text = format!(" {name} ");
        let width = to_u16(text.chars().count());
        let rect = Rect {
            x,
            y: tabs.y,
            width: width.min(tabs.right().saturating_sub(x)),
            height: 1,
        };
        // with the keyboard on the tabs, the current one is yellow
        let mut style = match (*tab == app.settings_page.tab, tabs_focused) {
            (true, true) => Style::new().black().on_yellow().bold(),
            (true, false) => Style::new().reversed().bold(),
            _ => Style::new().dark_gray(),
        };
        let mut line = Line::from(Span::styled(text, style));
        if app.hits.add(rect, Target::SettingsTab(to_u8(index))) {
            style = hover_row(style);
            line = hover_line(line, 0);
        }
        f.render_widget(Paragraph::new(line).style(style), rect);
        x += width + 1;
    }
}

/// Draws the Settings page: the frame, the tab strip and the current tab.
// Long function (reason verified, phase 6): frame, tab strip position (top or bottom) and tab dispatch are decided from the same inner rectangle in one top-to-bottom pass.
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
        let [tabs, _, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        (tabs, body)
    } else {
        let [body, _, tabs] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        (tabs, body)
    };
    draw_tab_strip(f, app, tabs);
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

/// The width of `text` in cells, as a terminal size.
fn text_width(text: &str) -> u16 {
    to_u16(text.chars().count())
}

/// The Save and Discard buttons on the frame's bottom edge, while there are
/// unsaved changes; drawn with the live keymap (keys change only on Save).
// Long function (reason verified, phase 6): the note and two buttons are placed left to right in one row; each x comes from the width of the previous one.
pub fn draw_save_buttons(f: &mut Frame, app: &mut App, area: Rect, changes: usize) {
    if changes == 0 || area.height < 2 {
        return;
    }
    let key = |action: Action| {
        app.keymap
            .key_for(&[Context::Settings, Context::Global], action)
            .map(|k| crate::ui::widgets::friendly(&k))
            .unwrap_or_default()
    };
    let note = format!(" ● {changes} unsaved ");
    let save = format!(" {} Save ", key(Action::Save));
    let discard = format!(" {} Discard ", key(Action::Discard));
    let total = text_width(&note) + text_width(&save) + text_width(&discard) + 2;
    let y = area.bottom() - 1;
    let mut x = area.right().saturating_sub(total + 2);
    f.render_widget(
        Paragraph::new(Span::styled(note.clone(), Style::new().yellow())),
        Rect {
            x,
            y,
            width: text_width(&note),
            height: 1,
        },
    );
    x += text_width(&note) + 1;
    for (text, id, base) in [
        (save, 's', Style::new().black().on_yellow().bold()),
        (discard, 'd', Style::new().reversed()),
    ] {
        let rect = Rect {
            x,
            y,
            width: text_width(&text),
            height: 1,
        };
        let style = if app.hits.add(rect, Target::SettingsButton(id)) {
            hover(base)
        } else {
            base
        };
        f.render_widget(Paragraph::new(Span::styled(text.clone(), style)), rect);
        x += text_width(&text) + 1;
    }
}
