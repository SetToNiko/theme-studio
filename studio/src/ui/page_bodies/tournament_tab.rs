//! Drawing the Fonts page's Tournament tab (`fonts/tournament_page.rs`): the setup, the
//! previews being fetched, a match, the results, and the bracket.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;
use crate::app::state::hit::{Target, cursor_line, hover_line};
use crate::fonts::pages::tournament::{POOLS, SIZES, SetupRow, Stage};
use crate::model::tournament::{Status, Tournament};
use crate::ui::widgets::to_u8;

pub fn draw_tournament(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tour().stage {
        Stage::Setup => draw_setup(f, app, area),
        Stage::Fetching => draw_fetching(f, app, area),
        Stage::Playing => draw_match(f, app, area),
        Stage::Results => draw_results(f, app, area),
    }
    if app.tour().bracket {
        draw_bracket(f, app, area);
    }
}

/// A clickable row with the cursor (text only) on `on`.
fn row(f: &mut Frame, app: &mut App, r: Rect, i: usize, line: Line<'static>, on: bool) {
    let line = if on {
        cursor_line(line, Style::new().reversed())
    } else if app.hits.add(r, Target::TourRow(i)) {
        hover_line(line, 0)
    } else {
        line
    };
    if on {
        app.hits.add(r, Target::TourRow(i));
    }
    f.render_widget(Paragraph::new(line), r);
}

/// One line of the setup list, and the row index when it is a clickable row.
type SetupItem = (Line<'static>, Option<usize>);

/// The line of a past tournament: its title, start and how it ended.
fn past_line(t: &Tournament) -> Line<'static> {
    let state = if let Some(winner) = t.winner() {
        format!("won by {}", t.entrants[winner].name)
    } else {
        let status = t.status();
        format!("unfinished: round {} of {}", status.round, status.rounds)
    };
    Line::from(vec![
        Span::raw(format!("   {:<34}", t.title)),
        Span::styled(format!("{}  {state}", t.started), Style::new().dark_gray()),
    ])
}

/// The "Pool size" row with its blank line before it, and the "Past tournaments" title when `has_past`.
fn size_items(index: usize, size: usize, has_past: bool) -> Vec<SetupItem> {
    let mut items: Vec<SetupItem> = Vec::new();
    items.push((Line::raw(""), None));
    let value = if size == 0 {
        "all of them".to_string()
    } else {
        format!("{size} at random")
    };
    items.push((
        Line::from(vec![
            Span::raw(format!("   {:<26}", "Pool size")),
            Span::styled(format!("‹ {value} ›"), Style::new().yellow()),
        ]),
        Some(index),
    ));
    if has_past {
        items.push((Line::raw(""), None));
        items.push((
            Line::styled(" Past tournaments (d deletes)", Style::new().bold()),
            None,
        ));
    }
    items
}

/// All lines of the setup list (pools, pool size, past tournaments) and the
/// number of clickable rows.
fn setup_items(app: &mut App) -> (Vec<SetupItem>, usize) {
    let rows = app.tour_setup_rows();
    let catalogue = app.dl.catalogue.is_some();
    let counts: Vec<usize> = POOLS.iter().map(|(k, _)| app.tour_pool(k).len()).collect();
    let size = SIZES[app.tour().size];
    let past = app.tour_past().clone();
    let mut items: Vec<SetupItem> = vec![(Line::styled(" Pick a pool", Style::new().bold()), None)];
    for (index, row) in rows.iter().enumerate() {
        match row {
            SetupRow::Pool(pool) => {
                let (key, label) = POOLS[*pool];
                let fonts = if key.starts_with("installed") || catalogue {
                    format!("{} fonts", counts[*pool])
                } else {
                    "loading…".into()
                };
                items.push((
                    Line::from(vec![
                        Span::raw(format!("   {label:<26}")),
                        Span::styled(fonts, Style::new().dark_gray()),
                    ]),
                    Some(index),
                ));
            }
            SetupRow::Size => items.extend(size_items(index, size, !past.is_empty())),
            SetupRow::Past(k) => items.push((past_line(&past[*k].1), Some(index))),
        }
    }
    (items, rows.len())
}

/// The "How it works" text beside (or under) the setup list.
fn draw_setup_about(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::styled("How it works", Style::new().bold()),
        Line::raw(""),
        Line::raw(
            "Every font of the pool is shuffled into a bracket. Each match shows the same text in two fonts, names hidden: pick the one you like (←/→ or a click) and it goes on. An odd one out gets a bye.",
        ),
        Line::raw(""),
        Line::raw(
            "It takes one pick fewer than there are fonts. The end shows a top 10 by how far each font got; from there install one, use it for a role, or fill every role from its family.",
        ),
        Line::raw(""),
        Line::raw("Tournaments are saved as you go: leave with Esc, pick one up again here."),
    ];
    f.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(
                Block::new()
                    .borders(Borders::LEFT)
                    .border_style(Style::new().dark_gray())
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .dark_gray(),
        area,
    );
}

fn draw_setup(f: &mut Frame, app: &mut App, area: Rect) {
    let [list, about] = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(55), Constraint::Min(30)]).areas(area)
    } else {
        Layout::vertical([Constraint::Min(8), Constraint::Length(7)]).areas(area)
    };
    let (items, row_count) = setup_items(app);
    let sel = app.tour().sel.min(row_count.saturating_sub(1));
    let focused = !app.dl.tabs_focused && !app.layout.page_tabs_focused;
    for (y, (line, clickable)) in (list.y..list.bottom()).zip(items) {
        let rect = Rect::new(list.x, y, list.width, 1);
        match clickable {
            Some(index) => row(f, app, rect, index, line, focused && index == sel),
            None => f.render_widget(Paragraph::new(line), rect),
        }
    }
    draw_setup_about(f, about);
}

fn draw_fetching(f: &mut Frame, app: &mut App, area: Rect) {
    let (ok, failed, total) = app.tour_fetched();
    let title = app.tour().pool.1.clone();
    let width = usize::from(area.width.saturating_sub(8).min(60));
    let filled = (ok + failed) * width / total.max(1);
    let lines = vec![
        Line::styled(format!(" {title}"), Style::new().bold()),
        Line::raw(""),
        Line::raw(format!(" Fetching the previews: {}/{total}", ok + failed)),
        Line::from(vec![
            " ".into(),
            Span::styled("█".repeat(filled), Style::new().yellow()),
            Span::styled("░".repeat(width - filled), Style::new().dark_gray()),
        ]),
        Line::raw(""),
        Line::styled(
            if failed > 0 {
                format!(" {failed} couldn't be fetched: they'll sit this one out.")
            } else {
                " The first match starts once they're all here.".to_string()
            },
            Style::new().dark_gray(),
        ),
        Line::styled(" Esc stops.", Style::new().dark_gray()),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

/// The head line of a match: the title and the round, match and fonts-left counts.
fn match_head_line(t: &Tournament, status: &Status) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {}", t.title), Style::new().bold()),
        Span::styled(
            format!(
                "   round {} of {} · match {} of {} · {} fonts left",
                status.round, status.rounds, status.game, status.games, status.left
            ),
            Style::new().dark_gray(),
        ),
    ])
}

/// The title of a match pane: the font's name (when the names are shown) or
/// "A" / "B", with an arrow that points to the side.
fn side_title(t: &Tournament, side: usize, names: bool, pairing: Option<(usize, usize)>) -> String {
    let who = pairing.map(|(left, right)| if side == 0 { left } else { right });
    let label = match (names, who) {
        (true, Some(entrant)) => t.entrants[entrant].name.clone(),
        _ => if side == 0 { "A" } else { "B" }.to_string(),
    };
    if side == 0 {
        format!(" ← {label} ")
    } else {
        format!(" {label} → ")
    }
}

/// Draws a match: the head line and the two panes (side by side or stacked).
fn draw_match(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(t) = app.tour().t.clone() else {
        return;
    };
    let names = app.tour().names;
    let [head, panes] = Layout::vertical([Constraint::Length(2), Constraint::Min(4)]).areas(area);
    f.render_widget(Paragraph::new(match_head_line(&t, &t.status())), head);
    let [first, second] = if panes.width >= 100 {
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(panes)
    } else {
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(panes)
    };
    let pair = t.pairing();
    for (side, rect) in [first, second].into_iter().enumerate() {
        draw_pane(f, app, rect, side, side_title(&t, side, names, pair), true);
    }
}

/// A preview pane: its frame, its image (kept at its size while
/// resizing), a click target when `pick` (the match's panes).
// Long function (reason verified, phase 6): the pane's image state is read, updated and read again in order, because the settle size must be set before the image is drawn.
fn draw_pane(f: &mut Frame, app: &mut App, area: Rect, index: usize, title: String, pick: bool) {
    let hovered = pick && app.hits.add(area, Target::TourPane(to_u8(index)));
    let block = Block::bordered()
        .title(Span::styled(title, Style::new().yellow().bold()))
        .border_style(if hovered {
            Style::new().yellow()
        } else {
            Style::new().dark_gray()
        });
    let inner = block.inner(area);
    f.render_widget(block, area);
    let pane = &mut app.tour().panes[index];
    let place = pane.settle.place(inner);
    pane.area = pane.settle.size((inner.width, inner.height));
    let due = pane.settle.due();
    let error = pane.error.clone();
    app.redraw_at = crate::model::settle::sooner(app.redraw_at, due);
    let pane = &mut app.tour().panes[index];
    match (pane.image.as_mut(), place) {
        (Some(img), Some(place)) => f.render_stateful_widget(
            ratatui_image::StatefulImage::default().resize(ratatui_image::Resize::Crop(None)),
            place,
            img,
        ),
        (None, _) => {
            let text = error.map_or(" drawing…".to_string(), |e| {
                format!(" Couldn't draw it: {e}")
            });
            f.render_widget(Paragraph::new(text).dark_gray(), inner);
        }
        _ => {}
    }
}

/// The installed font families, lower case and without spaces, to tell which
/// results are installed already.
fn installed_families(app: &App) -> Vec<String> {
    app.fonts
        .installed
        .iter()
        .map(|font| font.family.to_lowercase().replace(' ', ""))
        .collect()
}

/// The result list: a heading, then each placing with its fonts.
// Long function (reason verified, phase 6): one running `y` for the heading, the placings and the fonts; each row's click index counts across all placings.
fn draw_results_list(f: &mut Frame, app: &mut App, list: Rect, t: &Tournament, sel: usize) {
    let focused = !app.dl.tabs_focused && !app.layout.page_tabs_focused;
    let installed = installed_families(app);
    let mut y = list.y;
    let header = Line::styled(format!(" {}: the top 10", t.title), Style::new().bold());
    f.render_widget(Paragraph::new(header), Rect::new(list.x, y, list.width, 1));
    y += 2;
    let mut index = 0;
    for (label, who) in t.top() {
        if y >= list.bottom() {
            break;
        }
        f.render_widget(
            Paragraph::new(Line::styled(format!(" {label}"), Style::new().yellow())),
            Rect::new(list.x, y, list.width, 1),
        );
        y += 1;
        for entrant in who {
            if y >= list.bottom() {
                break;
            }
            let e = &t.entrants[entrant];
            let name = e.name.to_lowercase().replace(' ', "");
            let have = e.installed || installed.iter().any(|family| family.starts_with(&name));
            let line = Line::from(vec![
                Span::raw(format!("   {:<30}", e.name)),
                Span::styled(if have { "installed" } else { "" }, Style::new().green()),
            ]);
            let rect = Rect::new(list.x, y, list.width, 1);
            row(f, app, rect, index, line, focused && index == sel);
            y += 1;
            index += 1;
        }
        y += 1;
    }
}

fn draw_results(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(t) = app.tour().t.clone() else {
        return;
    };
    let [list, preview] = if area.width >= 100 {
        Layout::horizontal([Constraint::Length(48), Constraint::Min(30)]).areas(area)
    } else {
        Layout::vertical([Constraint::Length(16), Constraint::Min(6)]).areas(area)
    };
    let sel = app.tour().sel;
    draw_results_list(f, app, list, &t, sel);
    let title = t
        .top()
        .into_iter()
        .flat_map(|(_, v)| v)
        .nth(sel)
        .map_or(" Preview ".to_string(), |w| {
            format!(" {} ", t.entrants[w].name)
        });
    draw_pane(f, app, preview, 0, title, false);
}

/// The bracket text: every round, and who won each match.
fn bracket_lines(t: &Tournament) -> Vec<Line<'static>> {
    let name = |index: usize| t.entrants[index].name.clone();
    let mut lines: Vec<Line> = Vec::new();
    for (round, matches) in t.rounds.iter().enumerate() {
        let fonts: usize = matches.iter().map(|m| 1 + usize::from(m.b.is_some())).sum();
        lines.push(Line::styled(
            format!(" Round {} ({fonts} fonts)", round + 1),
            Style::new().yellow().bold(),
        ));
        for game in matches {
            let line = match (game.b, game.winner) {
                (None, _) => Line::from(vec![
                    Span::raw(format!("   {}", name(game.a))),
                    Span::styled("  (bye)", Style::new().dark_gray()),
                ]),
                (Some(other), Some(winner)) => Line::from(vec![
                    Span::raw(format!("   {:<28} vs {:<28}", name(game.a), name(other))),
                    Span::styled(format!(" → {}", name(winner)), Style::new().green()),
                ]),
                (Some(other), None) => Line::from(vec![
                    Span::raw(format!("   {:<28} vs {:<28}", name(game.a), name(other))),
                    Span::styled(" → ?", Style::new().dark_gray()),
                ]),
            };
            lines.push(line);
        }
        lines.push(Line::raw(""));
    }
    lines
}

/// `b`: every round so far, and who won each match.
fn draw_bracket(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(t) = app.tour().t.clone() else {
        return;
    };
    let lines = bracket_lines(&t);
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    let max = lines
        .len()
        .saturating_sub(usize::from(rect.height.saturating_sub(2)));
    let tour = app.tour();
    tour.bracket_scroll = tour.bracket_scroll.min(max);
    let shown: Vec<Line> = lines.into_iter().skip(tour.bracket_scroll).collect();
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(shown).block(
            Block::bordered()
                .title(" Bracket (j/k scroll, b or Esc closes) ")
                .border_style(Style::new().yellow()),
        ),
        rect,
    );
}
