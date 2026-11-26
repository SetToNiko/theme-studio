//! The README tab of the Settings page: the user guide, rendered for the pane.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::state::hit::Target;

/// docs/theme-studio.md, rendered for the pane (at most 100 columns wide, so
/// lines stay readable), from the scroll position, with a scrollbar.
// Long function: loads the guide, renders it and draws it with the scrollbar in one pass over the page state.
pub(super) fn draw_readme(f: &mut Frame, app: &mut App, area: Rect) {
    let path = crate::model::paths::guide(&app.store.share);
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let p = &mut app.settings_page;
    // read it (again, if the file changed)
    if p.readme.as_ref().is_none_or(|(m, _)| *m != modified) {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()));
        p.readme = Some((modified, text));
        p.readme_doc = None;
    }
    let text = match &p.readme {
        Some((_, Ok(t))) => t,
        Some((_, Err(e))) => {
            let line = Line::styled(format!(" {e}"), Style::new().fg(Color::Red));
            f.render_widget(Paragraph::new(line), area);
            return;
        }
        None => return,
    };
    let width = area.width.saturating_sub(3).min(100);
    if p.readme_doc.as_ref().is_none_or(|(w, _)| *w != width) {
        p.readme_doc = Some((width, crate::settings_page::readme::render(text, width)));
    }
    let doc = &p.readme_doc.as_ref().unwrap().1;
    let last = doc.lines.len().saturating_sub(1);
    p.readme_scroll = p.readme_scroll.min(last);
    let h = area.height as usize;
    let lines: Vec<Line> = doc
        .lines
        .iter()
        .skip(p.readme_scroll)
        .take(h)
        .cloned()
        .collect();
    let text_area = Rect {
        x: area.x + 1,
        width: width.min(area.width.saturating_sub(2)),
        ..area
    };
    f.render_widget(Paragraph::new(lines), text_area);
    // the wheel scrolls it
    app.hits.add(area, Target::SettingsDiff);
    if doc.lines.len() > h {
        let mut state = ratatui::widgets::ScrollbarState::new(doc.lines.len().saturating_sub(h))
            .position(p.readme_scroll);
        f.render_stateful_widget(
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area,
            &mut state,
        );
    }
}
