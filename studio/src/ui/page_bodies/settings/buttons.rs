//! The Save and Discard buttons of the Settings page.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::state::hit::{Target, hover};
use crate::keymap::action::Action;
use crate::keymap::context::Context;
use crate::ui::widgets::{friendly, to_u16};

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
            .map(|k| friendly(&k))
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
