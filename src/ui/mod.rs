mod main_view;
mod modals;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::{App, TextInput};

pub mod theme {
    use super::*;

    pub const ACCENT: Color = Color::Cyan;
    pub const ENABLED: Color = Color::Green;
    pub const DISABLED: Color = Color::DarkGray;
    pub const WARN: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;
    pub const GROUP: Color = Color::Magenta;
    pub const IP: Color = Color::LightBlue;
    pub const MUTED: Color = Color::Gray;

    pub fn key() -> Style {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    }

    pub fn border(active: bool) -> Style {
        if active {
            Style::new().fg(ACCENT)
        } else {
            Style::new().fg(DISABLED)
        }
    }

    pub fn selected(active: bool) -> Style {
        if active {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new().add_modifier(Modifier::BOLD)
        }
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let [header, body, status, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    main_view::draw_header(f, app, header);
    main_view::draw_body(f, app, body);
    main_view::draw_status(f, app, status);
    main_view::draw_keybar(f, app, keys);

    let mut modal = app.modal.take();
    if let Some(m) = modal.as_mut() {
        modals::draw(f, app, m);
    }
    app.modal = modal;
}

/// `key description` pairs for the bottom bar and dialogs.
pub fn hints(pairs: &[(&str, String)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(key.to_string(), theme::key()));
        spans.push(Span::raw(format!(" {desc}")));
    }
    Line::from(spans)
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2)).max(1);
    let h = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// Widgets drop `\t`, so tabs are expanded to 8-column stops for display.
pub fn expand_tabs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut col = 0;
    for c in text.chars() {
        if c == '\t' {
            let n = 8 - col % 8;
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(c);
            col += 1;
        }
    }
    out
}

/// Renders the visible window of an input and returns the cursor column offset.
pub fn input_window(input: &TextInput, width: usize) -> (String, usize) {
    let width = width.max(1);
    let start = input.cursor.saturating_sub(width - 1);
    let text: String = input.value.chars().skip(start).take(width).collect();
    (text, input.cursor - start)
}

/// The hosts field shows one name per row: each space starts a new one. Rows are scrolled so
/// the cursor stays within `height`. Returns the visible rows and the cursor as (column, row).
pub fn host_rows(input: &TextInput, width: usize, height: usize) -> (Vec<String>, (usize, usize)) {
    let width = width.max(2);
    let words: Vec<&str> = input.value.split(' ').collect();
    let before: String = input.value.chars().take(input.cursor).collect();
    let row = before.matches(' ').count();
    let col = before.chars().rev().take_while(|&c| c != ' ').count();
    let top = row.saturating_sub(height.max(1) - 1);
    let mut cursor = (col, row - top);
    let rows = (top..words.len())
        .take(height)
        .map(|r| {
            let word = words[r];
            if word.chars().count() <= width {
                word.to_string()
            } else if r == row {
                // An overlong name scrolls sideways around the cursor.
                let (text, c) = input_window(
                    &TextInput {
                        value: word.to_string(),
                        cursor: col,
                    },
                    width,
                );
                cursor.0 = c;
                text
            } else {
                let cut: String = word.chars().take(width - 1).collect();
                format!("{cut}…")
            }
        })
        .collect();
    (rows, cursor)
}

#[cfg(test)]
mod tests {
    #[test]
    fn expand_tabs() {
        assert_eq!(super::expand_tabs("127.0.0.1\tlocalhost"), "127.0.0.1       localhost");
        assert_eq!(super::expand_tabs("a\tb\t"), "a       b       ");
    }

    #[test]
    fn host_rows() {
        let input = crate::app::TextInput::new("aa.local bb.local c");
        let (rows, cursor) = super::host_rows(&input, 10, 3);
        assert_eq!(rows, ["aa.local", "bb.local", "c"]);
        assert_eq!(cursor, (1, 2));
        // Scrolled to keep the cursor visible.
        let (rows, cursor) = super::host_rows(&input, 10, 2);
        assert_eq!(rows, ["bb.local", "c"]);
        assert_eq!(cursor, (1, 1));
        // A trailing space opens an empty row for the next name.
        let (rows, cursor) = super::host_rows(&crate::app::TextInput::new("a "), 10, 3);
        assert_eq!(rows, ["a", ""]);
        assert_eq!(cursor, (0, 1));
        // Overlong names: shortened, or scrolled when the cursor is in them.
        let mut input = crate::app::TextInput::new("abcdefgh x");
        let (rows, _) = super::host_rows(&input, 5, 3);
        assert_eq!(rows, ["abcd…", "x"]);
        input.cursor = 8;
        let (rows, cursor) = super::host_rows(&input, 5, 3);
        assert_eq!((rows[0].as_str(), cursor), ("efgh", (4, 0)));
    }
}
