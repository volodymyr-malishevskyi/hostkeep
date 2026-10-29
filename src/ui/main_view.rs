use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Row, Table};

use super::{expand_tabs, hints, input_window, theme};
use crate::app::{App, Focus, StatusKind};
use crate::hosts::model::Line as HostLine;
use crate::paths::APP_NAME;
use crate::t;

const NARROW: u16 = 70;

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mut right = Vec::new();
    if !app.writable {
        right.push(Span::styled(
            format!("🔒 {}  ", t!("header-needs-sudo")),
            Style::new().fg(theme::WARN),
        ));
    }
    if app.is_dirty() {
        right.push(Span::styled(
            format!("● {}  ", t!("header-modified")),
            Style::new().fg(theme::WARN).add_modifier(Modifier::BOLD),
        ));
    }
    right.push(Span::styled(
        format!("{} ", crate::i18n::lang().code().to_uppercase()),
        Style::new().fg(theme::MUTED),
    ));
    let right = Line::from(right);

    // The path gets whatever is left and is shortened from the left: the file name matters most.
    let prefix = format!(" {APP_NAME} ─ ");
    let room = (area.width as usize).saturating_sub(prefix.chars().count() + right.width() + 1);
    let path = app.path.display().to_string();
    let count = path.chars().count();
    let path = if count > room {
        let tail: String = path.chars().skip(count + 1 - room.max(1)).collect();
        format!("…{tail}")
    } else {
        path
    };
    let left = Line::from(vec![
        Span::styled(format!(" {APP_NAME} "), theme::key()),
        Span::styled("─ ", Style::new().fg(theme::DISABLED)),
        Span::raw(path),
    ]);
    f.render_widget(Paragraph::new(left), area);
    f.render_widget(Paragraph::new(right).alignment(Alignment::Right), area);
}

pub fn draw_body(f: &mut Frame, app: &mut App, area: Rect) {
    let groups = app.groups();
    let labels: Vec<(String, usize, usize)> = std::iter::once((
        t!("group-all"),
        app.doc.entries().filter(|(_, e)| e.enabled).count(),
        app.doc.entries().count(),
    ))
    .chain(
        groups
            .iter()
            .map(|g| (App::group_label(g), g.enabled_count(&app.doc), g.entries.len())),
    )
    .collect();

    let (groups_area, entries_area) = if area.width < NARROW {
        // Narrow terminal: only the focused panel is shown.
        match app.focus {
            Focus::Groups => (Some(area), None),
            Focus::Entries => (None, Some(area)),
        }
    } else {
        let longest = labels.iter().map(|(l, _, _)| l.chars().count()).max().unwrap_or(10) as u16;
        let width = (longest + 14).clamp(22, 34);
        let [g, e] = Layout::horizontal([Constraint::Length(width), Constraint::Fill(1)]).areas(area);
        (Some(g), Some(e))
    };

    if let Some(area) = groups_area {
        draw_groups(f, app, area, &labels);
    }
    if let Some(area) = entries_area {
        draw_entries(f, app, area);
    }
}

fn draw_groups(f: &mut Frame, app: &mut App, area: Rect, labels: &[(String, usize, usize)]) {
    let active = app.focus == Focus::Groups;
    let inner_width = area.width.saturating_sub(4) as usize;
    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, (name, enabled, total))| {
            let count = format!("{enabled}/{total}");
            let name: String = name.chars().take(inner_width.saturating_sub(count.len() + 1)).collect();
            let pad = inner_width.saturating_sub(name.chars().count() + count.len());
            let name_style = if i == 0 {
                Style::new().add_modifier(Modifier::ITALIC)
            } else {
                Style::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(name, name_style),
                Span::raw(" ".repeat(pad)),
                Span::styled(count, Style::new().fg(theme::MUTED)),
            ]))
        })
        .collect();
    let block = Block::bordered()
        .title(format!(" {} ", t!("panel-groups")))
        .border_style(theme::border(active));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::selected(active))
        .highlight_symbol("▶ ");
    app.groups_state.select(Some(app.group_sel));
    f.render_stateful_widget(list, area, &mut app.groups_state);
}

fn draw_entries(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Entries;
    let visible = app.visible_entries();
    let group = app.selected_group();
    let show_group_col = group.is_none();

    let title = group.as_ref().map(App::group_label).unwrap_or_else(|| t!("group-all"));
    let (enabled, total) = match &group {
        Some(g) => (g.enabled_count(&app.doc), g.entries.len()),
        None => (
            app.doc.entries().filter(|(_, e)| e.enabled).count(),
            app.doc.entries().count(),
        ),
    };
    let mut block = Block::bordered()
        .title(format!(" {title} "))
        .title_top(Line::from(format!(" {} ", t!("enabled-of", enabled = enabled, total = total))).right_aligned())
        .border_style(theme::border(active));
    if !app.filter.value.is_empty() {
        block = block.title_bottom(
            Line::from(format!(" /{}  ({}) ", app.filter.value, visible.len())).style(Style::new().fg(theme::ACCENT)),
        );
    }

    if visible.is_empty() {
        let msg = if app.filter.value.is_empty() {
            t!("empty-group")
        } else {
            t!("empty-filter")
        };
        let inner = block.inner(area);
        f.render_widget(block, area);
        let y = inner.y + inner.height / 2;
        f.render_widget(
            Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(Style::new().fg(theme::MUTED)),
            Rect { y, height: 1, ..inner },
        );
        return;
    }

    let ip_width = visible
        .iter()
        .filter_map(|&i| app.doc.entry(i))
        .map(|e| e.ip.chars().count())
        .chain([t!("col-ip").chars().count()])
        .max()
        .unwrap_or(9)
        .clamp(7, 39) as u16;

    let group_names: Vec<String> = if show_group_col {
        visible
            .iter()
            .map(|&i| {
                let start = app.doc.group_of(i);
                match start.and_then(|s| app.doc.lines.get(s)) {
                    Some(HostLine::GroupStart { name, .. }) => name.clone(),
                    _ => t!("group-ungrouped"),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let group_width = if show_group_col {
        group_names
            .iter()
            .chain([&t!("col-group")])
            .map(|n| n.chars().count())
            .max()
            .unwrap_or(5)
            .min(24)
    } else {
        0
    };
    // Hosts and description share what is left 3:2. Inner width minus the highlight symbol,
    // the three flag columns, IP, group and the gaps between columns.
    let columns = if show_group_col { 7 } else { 6 };
    let rest = (area.width as usize).saturating_sub(2 + 1 + 3 + ip_width as usize + group_width + (columns - 1));
    let hosts_width = rest * 3 / 5;
    let comment_width = rest - hosts_width;
    app.hosts_width = hosts_width;

    let rows: Vec<Row> = visible
        .iter()
        .enumerate()
        .filter_map(|(row, &i)| app.doc.entry(i).map(|e| (row, i, e)))
        .map(|(row, i, e)| {
            let mark = if app.marked.contains(&i) {
                Span::styled("*", theme::key())
            } else {
                Span::raw(" ")
            };
            let state = if e.enabled {
                Span::styled("●", Style::new().fg(theme::ENABLED))
            } else {
                Span::styled("○", Style::new().fg(theme::DISABLED))
            };
            let flag = if app.report.conflict_lines.contains(&i) && e.enabled {
                Span::styled("⚠", Style::new().fg(theme::WARN))
            } else if app.report.duplicate_lines.contains(&i) {
                Span::styled("≡", Style::new().fg(theme::WARN))
            } else if e.is_protected() {
                Span::styled("◆", Style::new().fg(theme::MUTED))
            } else {
                Span::raw(" ")
            };
            let text_style = if e.enabled {
                Style::new()
            } else {
                Style::new().fg(theme::DISABLED)
            };
            let ip_style = if e.enabled {
                Style::new().fg(theme::IP)
            } else {
                text_style
            };
            let mut cells = vec![
                Line::from(mark),
                Line::from(state),
                Line::from(flag),
                Line::from(Span::styled(e.ip.clone(), ip_style)),
                Line::from(Span::styled(fit_hosts(&e.hosts, hosts_width), text_style)),
                Line::from(Span::styled(
                    expand_tabs(e.comment.as_deref().unwrap_or_default()),
                    Style::new().fg(theme::MUTED),
                )),
            ];
            if show_group_col {
                cells.push(Line::from(Span::styled(
                    group_names[row].clone(),
                    Style::new().fg(theme::GROUP),
                )));
            }
            Row::new(cells)
        })
        .collect();

    let mut widths = vec![
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(ip_width),
        Constraint::Length(hosts_width as u16),
        Constraint::Length(comment_width as u16),
    ];
    let mut header = vec![
        String::new(),
        String::new(),
        String::new(),
        t!("col-ip"),
        t!("col-hosts"),
        t!("col-comment"),
    ];
    if show_group_col {
        widths.push(Constraint::Length(group_width as u16));
        header.push(t!("col-group"));
    }

    let table = Table::new(rows, widths)
        .header(Row::new(header).style(Style::new().fg(theme::MUTED).add_modifier(Modifier::BOLD)))
        .block(block)
        .column_spacing(1)
        .row_highlight_style(theme::selected(active))
        .highlight_symbol("▶");

    app.page_size = area.height.saturating_sub(4).max(1) as usize;
    app.entries_state.select(Some(app.entry_sel));
    f.render_stateful_widget(table, area, &mut app.entries_state);
}

/// Joins as many whole host names as fit into `width`, then `+N` for the rest.
fn fit_hosts(hosts: &[String], width: usize) -> String {
    let all = hosts.join(" ");
    if all.chars().count() <= width {
        return all;
    }
    for shown in (1..hosts.len()).rev() {
        let text = format!("{} +{}", hosts[..shown].join(" "), hosts.len() - shown);
        if text.chars().count() <= width {
            return text;
        }
    }
    // Not even the first name fits: shorten it.
    let more = if hosts.len() > 1 {
        format!(" +{}", hosts.len() - 1)
    } else {
        String::new()
    };
    let keep = width.saturating_sub(more.chars().count() + 1);
    let first: String = hosts[0].chars().take(keep).collect();
    format!("{first}…{more}")
}

pub fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let warn = Style::new().fg(theme::WARN);
    let line = if let Some((text, kind)) = &app.status {
        let style = match kind {
            StatusKind::Info => Style::new().fg(theme::ENABLED),
            StatusKind::Warn => warn,
            StatusKind::Error => Style::new().fg(theme::ERROR).add_modifier(Modifier::BOLD),
        };
        Line::from(Span::styled(format!(" {text}"), style))
    } else if let Some(e) = app.current_entry().and_then(|i| app.doc.entry(i).map(|e| (i, e))) {
        let (i, entry) = e;
        if let Some(c) = app.report.conflict_for(&entry.hosts).filter(|_| entry.enabled) {
            Line::from(Span::styled(
                format!(
                    " ⚠ {}",
                    t!("warn-conflict", host = c.host.as_str(), ips = c.ips.join(", "))
                ),
                warn,
            ))
        } else if app.report.duplicate_lines.contains(&i) {
            Line::from(Span::styled(format!(" ≡ {}", t!("warn-duplicate")), warn))
        } else if entry.hosts.join(" ").chars().count() > app.hosts_width {
            let all = entry.hosts.join(" ");
            let room = (area.width as usize).saturating_sub(1);
            let text = if all.chars().count() > room {
                let cut: String = all.chars().take(room.saturating_sub(1)).collect();
                format!("{cut}…")
            } else {
                all
            };
            Line::from(Span::raw(format!(" {text}")))
        } else {
            summary_line(app)
        }
    } else {
        summary_line(app)
    };
    f.render_widget(Paragraph::new(line), area);
}

fn summary_line(app: &App) -> Line<'static> {
    let warn = Style::new().fg(theme::WARN);
    let invalid = app.doc.lines.iter().filter(|l| matches!(l, HostLine::Other(_))).count();
    if !app.marked.is_empty() {
        Line::from(Span::styled(
            format!(" * {}", t!("status-marked", count = app.marked.len())),
            theme::key(),
        ))
    } else if !app.report.conflicts.is_empty() {
        Line::from(Span::styled(
            format!(" ⚠ {}", t!("warn-conflicts-total", count = app.report.conflicts.len())),
            warn,
        ))
    } else if invalid > 0 {
        Line::from(Span::styled(
            format!(" ⚠ {}", t!("warn-invalid-lines", count = invalid)),
            warn,
        ))
    } else {
        Line::from(Span::styled(
            format!(" {}", t!("entries-count", count = app.doc.entries().count())),
            Style::new().fg(theme::MUTED),
        ))
    }
}

pub fn draw_keybar(f: &mut Frame, app: &App, area: Rect) {
    if app.searching && app.modal.is_none() {
        let prefix = " / ";
        let width = (area.width as usize).saturating_sub(prefix.len() + 30).max(10);
        let (text, cursor) = input_window(&app.filter, width);
        let mut spans = vec![
            Span::styled(prefix, theme::key()),
            Span::raw(format!("{text:<width$}")),
            Span::raw("  "),
        ];
        spans.extend(hints(&[("Enter", t!("key-apply")), ("Esc", t!("key-clear"))]).spans);
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        f.set_cursor_position(Position::new(area.x + prefix.len() as u16 + cursor as u16, area.y));
        return;
    }
    let pairs: Vec<(&str, String)> = match app.focus {
        Focus::Groups => vec![
            ("Space", t!("key-toggle-group")),
            ("a", t!("key-new-group")),
            ("e", t!("key-rename")),
            ("d", t!("key-delete")),
            ("J/K", t!("key-move")),
            ("s", t!("key-save")),
            ("?", t!("key-help")),
        ],
        Focus::Entries => vec![
            ("Space", t!("key-toggle")),
            ("a", t!("key-add")),
            ("e", t!("key-edit")),
            ("d", t!("key-delete")),
            ("/", t!("key-search")),
            ("s", t!("key-save")),
            ("?", t!("key-help")),
        ],
    };
    let mut line = hints(&pairs);
    line.spans.insert(0, Span::raw(" "));
    f.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::fit_hosts;

    #[test]
    fn fits_host_names() {
        let hosts: Vec<String> = ["a.local", "bb.local", "ccc.local"].map(String::from).to_vec();
        assert_eq!(fit_hosts(&hosts, 30), "a.local bb.local ccc.local");
        assert_eq!(fit_hosts(&hosts, 19), "a.local bb.local +1");
        assert_eq!(fit_hosts(&hosts, 12), "a.local +2");
        assert_eq!(fit_hosts(&hosts, 8), "a.lo… +2");
        assert_eq!(fit_hosts(&hosts[..1], 5), "a.lo…");
    }
}
