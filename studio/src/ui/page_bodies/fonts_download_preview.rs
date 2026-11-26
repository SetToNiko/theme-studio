//! The preview box of the Download fonts page: the highlighted font drawn
//! with a sample text (`fonts/preview_page.rs`).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::app::state::hit::{Target, hover};
use crate::model::samples::langs::LANGS;
use crate::model::samples::{KINDS, Kind};
use crate::system::download::Entry;
use crate::ui::widgets::to_u16;

/// The title of the preview box, with the clickable dropdowns: " Preview  Code ▾
/// Rust ▾  12pt ". Returns the spans and the kind of sample, when a font is
/// highlighted.
fn preview_title(
    app: &mut App,
    area: Rect,
    selected: Option<&Entry>,
) -> (Vec<Span<'static>>, Option<Kind>) {
    let mut title: Vec<Span> = vec![" Preview ".into()];
    let (Some(entry), Some(pv)) = (selected, app.dl.preview.as_ref()) else {
        return (title, None);
    };
    let kind = pv.kind_for(entry);
    let label = KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("README", |(_, l)| crate::fonts::preview_page::kind_label(l));
    let mut menus = vec![(0u8, format!("{label} ▾"))];
    if kind == Kind::Code {
        menus.push((1, format!("{} ▾", LANGS[pv.lang].0)));
    }
    let size = pv.size;
    // the title starts after the corner
    let mut x = area.x + 1 + to_u16(title[0].width());
    for (menu, text) in menus {
        title.push(" ".into());
        x += 1;
        let width = to_u16(text.chars().count()).min(area.right().saturating_sub(x));
        let rect = Rect::new(x, area.y, width, 1);
        let base = Style::new().yellow();
        let hovered = app.hits.add(rect, Target::PreviewMenu(menu));
        title.push(Span::styled(text, if hovered { hover(base) } else { base }));
        x += width + 1;
        title.push(" ".into());
    }
    title.push(format!(" {size}pt ").into());
    (title, Some(kind))
}

/// Draws the preview image in `inner`, or the note when there is none.
fn draw_preview_image(f: &mut Frame, app: &mut App, inner: Rect) {
    // while resizing, the preview keeps the size it had (and isn't
    // re-rendered) until the size holds still
    let mut place = Some(inner);
    if let Some(pv) = app.dl.preview.as_mut() {
        place = pv.settle.place(inner);
        pv.area = pv.settle.size((inner.width, inner.height));
        app.redraw_at = crate::model::settle::sooner(app.redraw_at, pv.settle.due());
    }
    let note = app.preview_note();
    match app.dl.preview.as_mut().and_then(|pv| pv.image.as_mut()) {
        Some(img) => {
            if let Some(place) = place {
                f.render_stateful_widget(
                    ratatui_image::StatefulImage::default()
                        .resize(ratatui_image::Resize::Crop(None)),
                    place,
                    img,
                );
            }
        }
        None => f.render_widget(Paragraph::new(format!(" {note}")).dark_gray(), inner),
    }
}

/// The highlighted font drawn with a sample text (`fonts/preview_page.rs`).
pub(super) fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let selected = app.preview_entry();
    // " Preview  Code ▾  Rust ▾  12pt ": the kind and the language are
    // dropdowns
    let (title, kind) = preview_title(app, area, selected.as_ref());
    let mut hints = String::from(" t text  +/- size ");
    if kind == Some(Kind::Code) {
        hints += " L language  </> page ";
    }
    if kind == Some(Kind::Prose) {
        hints += " </> page ";
    }
    let block = Block::new()
        .borders(Borders::LEFT | Borders::TOP)
        .title(Line::from(title))
        .title_bottom(Line::styled(hints, Style::new().dark_gray()))
        .border_style(Style::new().dark_gray());
    let inner = block.inner(area);
    f.render_widget(block, area);
    // the wheel scrolls the text
    app.hits.add(inner, Target::DlPreview);
    if selected.is_none() {
        return;
    }
    draw_preview_image(f, app, inner);
}
