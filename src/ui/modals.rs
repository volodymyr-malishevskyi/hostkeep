use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Row, Table, Wrap};
use similar::{ChangeTag, TextDiff};

use super::{centered, expand_tabs, hints, host_rows, input_window, theme};
use crate::app::{App, BackupsView, Dialog, DiffView, Form, GroupPicker, Modal, NameInput, NamePurpose, TextInput};
use crate::hosts::model::Line as HostLine;
use crate::t;

pub fn draw(f: &mut Frame, app: &App, modal: &mut Modal) {
    match modal {
        Modal::Form(form) => draw_form(f, app, form),
        Modal::Dialog(dialog) => draw_dialog(f, dialog),
        Modal::Diff(diff) => draw_diff(f, app, diff),
        Modal::Raw { scroll } => draw_raw(f, app, scroll),
        Modal::Backups(view) => draw_backups(f, app, view),
        Modal::Help { scroll } => draw_help(f, scroll),
        Modal::GroupName(name) => draw_group_name(f, name),
        Modal::GroupPicker(picker) => draw_picker(f, app, picker),
    }
}

fn popup(f: &mut Frame, area: Rect, title: String) -> Rect {
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::new().fg(theme::ACCENT));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    inner
}

fn footer(f: &mut Frame, area: Rect, line: Line<'static>) {
    f.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        Rect {
            y: area.bottom().saturating_sub(1),
            height: 1,
            ..area
        },
    );
}

/// Clamps a scroll offset to the content and writes it back.
fn clamp_scroll(scroll: &mut u16, content: usize, height: u16) -> u16 {
    let max = content.saturating_sub(height as usize).min(u16::MAX as usize) as u16;
    *scroll = (*scroll).min(max);
    *scroll
}

// ---- form ----

/// Most rows the hosts field grows to (one name per row); beyond that it scrolls.
const HOSTS_MAX_ROWS: usize = 8;

fn draw_form(f: &mut Frame, app: &App, form: &mut Form) {
    let title = if form.editing.is_some() {
        t!("form-edit-title")
    } else {
        t!("form-add-title")
    };
    let labels = [
        t!("form-ip"),
        t!("form-hosts"),
        t!("form-comment"),
        t!("form-group"),
        t!("form-state"),
    ];
    let label_w = labels.iter().map(|l| l.chars().count()).max().unwrap_or(10) + 2;
    // Popup width minus borders, padding and brackets (the popup shrinks on narrow screens).
    let popup_w = centered(f.area(), 64, 1).width as usize;
    let field_w = popup_w.saturating_sub(label_w + 6).max(8);
    let hosts_rows = host_rows(&form.hosts, field_w, usize::MAX).0.len().min(HOSTS_MAX_ROWS);
    // Centered when opened; after that the top stays put and the form grows downwards,
    // moving up only as far as needed to stay on screen.
    let mut area = centered(f.area(), 64, 11 + hosts_rows as u16);
    let top = *form.top.get_or_insert(area.y);
    area.y = top.min(f.area().bottom().saturating_sub(area.height + 1));
    form.top = Some(area.y);
    let inner = popup(f, area, title);
    let groups = app.group_choices();

    // One row per field, except hosts, which wraps over several.
    let heights = [1, hosts_rows as u16, 1, 1, 1];
    let mut constraints = vec![Constraint::Length(1)];
    constraints.extend(heights.map(Constraint::Length));
    constraints.extend([Constraint::Length(1); 2]);
    let rows = Layout::vertical(constraints).split(Rect {
        x: inner.x + 1,
        width: inner.width.saturating_sub(2),
        ..inner
    });
    let inputs: [&TextInput; 3] = [&form.ip, &form.hosts, &form.comment];
    for (i, label) in labels.iter().enumerate() {
        let row = rows[i + 1];
        let focused = form.focus == i;
        let label_style = if focused { theme::key() } else { Style::new() };
        let mut spans = vec![Span::styled(format!("{label:<label_w$}"), label_style)];
        let field_style = if focused {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().fg(theme::MUTED)
        };
        match i {
            0..=2 => {
                let (texts, (col, line)) = if i == 1 {
                    // Unfocused, the field shows the names from the start.
                    let start = TextInput {
                        cursor: 0,
                        ..form.hosts.clone()
                    };
                    let input = if focused { &form.hosts } else { &start };
                    host_rows(input, field_w, row.height as usize)
                } else {
                    let (text, cursor) = input_window(inputs[i], field_w);
                    (vec![text], (cursor, 0))
                };
                let text_style = if focused {
                    Style::new().add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::new()
                };
                let mut lines = Vec::new();
                for n in 0..row.height as usize {
                    let text = texts.get(n).map(String::as_str).unwrap_or_default();
                    let lead = if n == 0 {
                        std::mem::take(&mut spans)
                    } else {
                        vec![Span::raw(" ".repeat(label_w))]
                    };
                    let mut line = lead;
                    line.push(Span::raw("["));
                    line.push(Span::styled(format!("{text:<field_w$}"), text_style));
                    line.push(Span::raw("]"));
                    lines.push(Line::from(line));
                }
                if focused {
                    f.set_cursor_position(Position::new(
                        row.x + label_w as u16 + 1 + col as u16,
                        row.y + line as u16,
                    ));
                }
                f.render_widget(Paragraph::new(lines), row);
                continue;
            }
            3 => {
                let name = groups.get(form.group).map(|(n, _)| n.clone()).unwrap_or_default();
                spans.push(Span::styled(format!("◀ {name} ▶"), field_style));
            }
            _ => {
                let check = if form.enabled { "[x]" } else { "[ ]" };
                spans.push(Span::styled(format!("{check} {}", t!("form-enabled")), field_style));
            }
        }
        f.render_widget(Paragraph::new(Line::from(spans)), row);
    }
    if let Some(err) = &form.error {
        f.render_widget(
            Paragraph::new(Span::styled(format!("✗ {err}"), Style::new().fg(theme::ERROR))),
            rows[7],
        );
    }
    footer(
        f,
        inner,
        hints(&[
            ("Enter", t!("btn-save")),
            ("Tab", t!("key-next-field")),
            ("Esc", t!("btn-cancel")),
        ]),
    );
}

// ---- dialogs ----

fn draw_dialog(f: &mut Frame, dialog: &Dialog) {
    let buttons_w: usize = dialog.buttons.iter().map(|b| b.chars().count() + 6).sum();
    let width = (dialog.message.chars().count().max(buttons_w) + 6).clamp(36, 72) as u16;
    let text_w = width.saturating_sub(4).max(1) as usize;
    let lines = dialog
        .message
        .lines()
        .map(|l| l.chars().count().div_ceil(text_w).max(1))
        .sum::<usize>() as u16;
    let area = centered(f.area(), width, lines + 6);
    let inner = popup(f, area, dialog.title.clone());
    let text_area = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: lines,
    };
    f.render_widget(
        Paragraph::new(dialog.message.clone()).wrap(Wrap { trim: false }),
        text_area,
    );

    let mut spans = Vec::new();
    for (i, b) in dialog.buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if i == dialog.selected {
            Style::new()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::new()
        };
        spans.push(Span::styled(format!("[ {b} ]"), style));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        Rect {
            y: inner.bottom().saturating_sub(2),
            height: 1,
            ..inner
        },
    );
}

fn draw_group_name(f: &mut Frame, name: &NameInput) {
    let title = match name.purpose {
        NamePurpose::Create => t!("group-new-title"),
        NamePurpose::Rename(_) => t!("group-rename-title"),
    };
    let area = centered(f.area(), 50, 7);
    let inner = popup(f, area, title);
    let width = inner.width.saturating_sub(4) as usize;
    let (text, cursor) = input_window(&name.input, width);
    let row = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("["),
            Span::styled(
                format!("{text:<width$}"),
                Style::new().add_modifier(Modifier::UNDERLINED),
            ),
            Span::raw("]"),
        ])),
        row,
    );
    f.set_cursor_position(Position::new(row.x + 1 + cursor as u16, row.y));
    if let Some(err) = &name.error {
        f.render_widget(
            Paragraph::new(Span::styled(format!("✗ {err}"), Style::new().fg(theme::ERROR))),
            Rect { y: row.y + 1, ..row },
        );
    }
    footer(f, inner, hints(&[("Enter", t!("btn-ok")), ("Esc", t!("btn-cancel"))]));
}

fn draw_picker(f: &mut Frame, app: &App, picker: &GroupPicker) {
    let choices = app.group_choices();
    let width = choices
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(10)
        .max(t!("picker-title").chars().count()) as u16
        + 10;
    let area = centered(f.area(), width, choices.len() as u16 + 4);
    let inner = popup(f, area, t!("picker-title"));
    let items: Vec<ListItem> = choices.into_iter().map(|(n, _)| ListItem::new(n)).collect();
    let list = List::new(items)
        .highlight_style(theme::selected(true))
        .highlight_symbol("▶ ");
    let mut state = ListState::default().with_selected(Some(picker.selected));
    f.render_stateful_widget(
        list,
        Rect {
            height: inner.height.saturating_sub(1),
            ..inner
        },
        &mut state,
    );
    footer(f, inner, hints(&[("Enter", t!("btn-ok")), ("Esc", t!("btn-cancel"))]));
}

// ---- diff / raw / backups / help ----

/// Unified diff lines with 3 lines of context, plus added/removed counts.
fn diff_lines(old: &str, new: &str) -> (Vec<Line<'static>>, usize, usize) {
    let diff = TextDiff::from_lines(old, new);
    let (mut lines, mut added, mut removed) = (Vec::new(), 0, 0);
    for (gi, group) in diff.grouped_ops(3).iter().enumerate() {
        if gi > 0 {
            lines.push(Line::from(Span::styled("   ⋯", Style::new().fg(theme::DISABLED))));
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let (sign, style, num) = match change.tag() {
                    ChangeTag::Delete => {
                        removed += 1;
                        ("-", Style::new().fg(theme::ERROR), change.old_index())
                    }
                    ChangeTag::Insert => {
                        added += 1;
                        ("+", Style::new().fg(theme::ENABLED), change.new_index())
                    }
                    ChangeTag::Equal => (" ", Style::new().fg(theme::MUTED), change.new_index()),
                };
                let text = expand_tabs(change.value().trim_end_matches(['\n', '\r']));
                let num = num.map(|n| format!("{:>4}", n + 1)).unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(num, Style::new().fg(theme::DISABLED)),
                    Span::styled(format!(" {sign} "), style),
                    Span::styled(text, style),
                ]));
            }
        }
    }
    (lines, added, removed)
}

fn big_area(f: &Frame) -> Rect {
    let a = f.area();
    centered(a, a.width.saturating_sub(4).max(40), a.height.saturating_sub(2).max(10))
}

fn draw_diff(f: &mut Frame, app: &App, diff: &mut DiffView) {
    let area = big_area(f);
    let inner = popup(f, area, t!("diff-title", path = app.path.display().to_string()));
    let (lines, added, removed) = diff_lines(&diff.old, &diff.new);
    let body = Rect {
        height: inner.height.saturating_sub(3),
        ..inner
    };
    let scroll = clamp_scroll(&mut diff.scroll, lines.len(), body.height);
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), body);

    let mut summary = vec![
        Span::styled(format!(" +{added}"), Style::new().fg(theme::ENABLED)),
        Span::styled(format!("  −{removed}"), Style::new().fg(theme::ERROR)),
        Span::raw(format!("   {}", t!("diff-backup-note"))),
    ];
    if !app.writable {
        summary.push(Span::styled(
            format!(" {}", t!("diff-sudo-note")),
            Style::new().fg(theme::WARN),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(summary)),
        Rect {
            y: inner.bottom().saturating_sub(2),
            height: 1,
            ..inner
        },
    );
    footer(
        f,
        inner,
        hints(&[
            ("Enter/y", t!("btn-save")),
            ("Esc/n", t!("btn-back")),
            ("↑↓ PgUp PgDn", t!("key-scroll")),
        ]),
    );
}

fn draw_raw(f: &mut Frame, app: &App, scroll: &mut u16) {
    let area = big_area(f);
    let inner = popup(f, area, t!("raw-title"));
    let num_w = app.doc.lines.len().to_string().len().max(3);
    let num = |i: usize| Span::styled(format!("{:>num_w$}  ", i + 1), Style::new().fg(theme::DISABLED));
    let lines: Vec<Line> = app
        .doc
        .lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let text = expand_tabs(&l.text());
            let mut spans = vec![num(i)];
            match l {
                HostLine::Entry(e) if e.enabled => {
                    // Color the IP, keep the rest of the original text as is.
                    let lead = text.len() - text.trim_start().len();
                    let ip_end = (lead + e.ip.len()).min(text.len());
                    let (rest, comment) = match text[ip_end..].find('#') {
                        Some(p) => (text[ip_end..ip_end + p].to_string(), text[ip_end + p..].to_string()),
                        None => (text[ip_end..].to_string(), String::new()),
                    };
                    spans.push(Span::styled(text[..ip_end].to_string(), Style::new().fg(theme::IP)));
                    spans.push(Span::raw(rest));
                    spans.push(Span::styled(comment, Style::new().fg(theme::MUTED)));
                }
                HostLine::Entry(_) => spans.push(Span::styled(text, Style::new().fg(theme::DISABLED))),
                HostLine::Comment(_) => spans.push(Span::styled(text, Style::new().fg(theme::MUTED))),
                HostLine::GroupStart { .. } | HostLine::GroupEnd(_) => spans.push(Span::styled(
                    text,
                    Style::new().fg(theme::GROUP).add_modifier(Modifier::BOLD),
                )),
                HostLine::Other(_) => spans.push(Span::styled(text, Style::new().fg(theme::ERROR))),
                HostLine::Blank(_) => spans.push(Span::raw(text)),
            }
            Line::from(spans)
        })
        .collect();
    let body = Rect {
        height: inner.height.saturating_sub(1),
        ..inner
    };
    let scroll = clamp_scroll(scroll, lines.len(), body.height);
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), body);
    footer(
        f,
        inner,
        hints(&[("↑↓ PgUp PgDn", t!("key-scroll")), ("Esc", t!("btn-close"))]),
    );
}

/// `hosts-20261029-114200-2` → `2026-10-29 11:42:00 (2)`.
fn backup_label(name: &str) -> String {
    let rest = name.strip_prefix(crate::hosts::writer::BACKUP_PREFIX).unwrap_or(name);
    let b = rest.as_bytes();
    if rest.len() >= 15 && b[8] == b'-' && rest[..8].bytes().chain(rest[9..15].bytes()).all(|c| c.is_ascii_digit()) {
        let suffix = rest.get(16..).map(|s| format!(" ({s})")).unwrap_or_default();
        format!(
            "{}-{}-{} {}:{}:{}{suffix}",
            &rest[..4],
            &rest[4..6],
            &rest[6..8],
            &rest[9..11],
            &rest[11..13],
            &rest[13..15]
        )
    } else {
        name.to_string()
    }
}

fn draw_backups(f: &mut Frame, app: &App, view: &mut BackupsView) {
    let area = big_area(f);
    let inner = popup(f, area, t!("backups-title"));
    let body = Rect {
        height: inner.height.saturating_sub(1),
        ..inner
    };
    let [list_area, diff_area] = Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)]).areas(body);

    let items: Vec<ListItem> = view
        .files
        .iter()
        .map(|p| {
            ListItem::new(backup_label(
                &p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ))
        })
        .collect();
    let list = List::new(items)
        .block(Block::bordered().border_style(theme::border(true)))
        .highlight_style(theme::selected(true))
        .highlight_symbol("▶ ");
    let mut state = ListState::default().with_selected(Some(view.selected));
    f.render_stateful_widget(list, list_area, &mut state);

    let backup = std::fs::read_to_string(&view.files[view.selected]).unwrap_or_default();
    let (lines, added, removed) = diff_lines(&app.doc.to_text(), &backup);
    let block = Block::bordered()
        .title(format!(" {} ", t!("backups-diff-title")))
        .title_top(Line::from(format!(" +{added} −{removed} ")).right_aligned())
        .border_style(theme::border(false));
    let diff_inner = block.inner(diff_area);
    f.render_widget(block, diff_area);
    if lines.is_empty() {
        f.render_widget(
            Paragraph::new(t!("backups-identical")).style(Style::new().fg(theme::MUTED)),
            diff_inner,
        );
    } else {
        let scroll = clamp_scroll(&mut view.scroll, lines.len(), diff_inner.height);
        f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), diff_inner);
    }
    footer(
        f,
        inner,
        hints(&[
            ("Enter", t!("btn-restore")),
            ("PgUp/PgDn", t!("key-scroll")),
            ("Esc", t!("btn-close")),
        ]),
    );
}

const HELP: &[(&str, &str)] = &[
    ("↑↓ / j k", "help-move"),
    ("← → / h l, Tab", "help-panels"),
    ("g / G, PgUp / PgDn", "help-jump"),
    ("Space", "help-toggle"),
    ("a", "help-add"),
    ("e / Enter", "help-edit"),
    ("d / Del", "help-delete"),
    ("c", "help-duplicate"),
    ("m", "help-move-group"),
    ("J / K, Shift+↑↓", "help-reorder"),
    ("v", "help-mark"),
    ("/", "help-search"),
    ("u / Ctrl+r", "help-undo"),
    ("s / Ctrl+s", "help-save"),
    ("r", "help-raw"),
    ("b", "help-backups"),
    ("L", "help-language"),
    ("?", "help-help"),
    ("q / Esc", "help-quit"),
];

fn draw_help(f: &mut Frame, scroll: &mut u16) {
    let area = centered(f.area(), 72, HELP.len() as u16 + 6);
    let inner = popup(f, area, t!("help-title"));
    let body = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(3),
    };
    let offset = clamp_scroll(scroll, HELP.len(), body.height) as usize;
    let rows: Vec<Row> = HELP
        .iter()
        .skip(offset)
        .map(|(k, d)| Row::new(vec![Span::styled(*k, theme::key()), Span::raw(t!(d))]))
        .collect();
    f.render_widget(Table::new(rows, [Constraint::Length(22), Constraint::Fill(1)]), body);
    footer(f, inner, hints(&[("↑↓", t!("key-scroll")), ("Esc", t!("btn-close"))]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_labels() {
        assert_eq!(backup_label("hosts-20261029-114200"), "2026-10-29 11:42:00");
        assert_eq!(backup_label("hosts-20261029-114200-3"), "2026-10-29 11:42:00 (3)");
        assert_eq!(backup_label("hosts-weird"), "hosts-weird");
    }

    #[test]
    fn diff_counts() {
        let (lines, added, removed) = diff_lines("a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!((added, removed), (2, 1));
        assert_eq!(lines.len(), 5);
    }
}
