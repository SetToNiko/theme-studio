//! The README tab of the Settings page: the user guide, rendered for the pane.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::app::App;
use crate::app::state::hit::Target;
use crate::settings_page::readme::Doc;
use crate::settings_page::state::SettingsPage;

/// Reads the guide at `path` again when the file changed, and drops the
/// rendered copy then.
fn refresh_readme(p: &mut SettingsPage, path: &std::path::Path) {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    if p.readme.as_ref().is_none_or(|(m, _)| *m != modified) {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
        p.readme = Some((modified, text));
        p.readme_doc = None;
    }
}

/// The lines of `doc` from `scroll`, `h` of them.
fn readme_lines(doc: &Doc, scroll: usize, h: usize) -> Vec<Line<'static>> {
    doc.lines.iter().skip(scroll).take(h).cloned().collect()
}

/// The scrollbar on the right edge of `area`, when the guide is longer than the pane.
fn draw_readme_scrollbar(f: &mut Frame, area: Rect, total: usize, scroll: usize) {
    let h = area.height as usize;
    if total <= h {
        return;
    }
    let mut state = ScrollbarState::new(total.saturating_sub(h)).position(scroll);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area,
        &mut state,
    );
}

/// docs/theme-studio.md, rendered for the pane (at most 100 columns wide, so
/// lines stay readable), from the scroll position, with a scrollbar.
pub(super) fn draw_readme(f: &mut Frame, app: &mut App, area: Rect) {
    let path = crate::model::paths::guide(&app.store.share);
    // read it (again, if the file changed)
    refresh_readme(&mut app.settings_page, &path);
    let p = &mut app.settings_page;
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
    p.readme_scroll = p.readme_scroll.min(doc.lines.len().saturating_sub(1));
    let lines = readme_lines(doc, p.readme_scroll, area.height as usize);
    let text_area = Rect {
        x: area.x + 1,
        width: width.min(area.width.saturating_sub(2)),
        ..area
    };
    f.render_widget(Paragraph::new(lines), text_area);
    let (total, scroll) = (doc.lines.len(), p.readme_scroll);
    // the wheel scrolls it
    app.hits.add(area, Target::SettingsDiff);
    draw_readme_scrollbar(f, area, total, scroll);
}
