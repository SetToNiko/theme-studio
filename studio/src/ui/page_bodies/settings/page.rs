//! The Settings page: the frame, the tab strip and the current tab.
//!
//! Everything clickable goes into the hit map with a hover look.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::{behaviour, history, keymaps, readme};
use crate::app::app_state::App;
use crate::app::state::hit::{Target, hover_line, hover_row};
use crate::settings_page::state::{TABS, Tab};
use crate::ui::widgets::{to_u8, to_u16};

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
