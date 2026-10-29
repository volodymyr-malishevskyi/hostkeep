use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Text field. `cursor` counts chars, not bytes.
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            cursor: value.chars().count(),
        }
    }

    fn byte_pos(&self, char_pos: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_pos)
            .map_or(self.value.len(), |(i, _)| i)
    }

    /// Returns true if the key was consumed.
    pub fn handle(&mut self, key: KeyEvent) -> bool {
        let len = self.value.chars().count();
        let word = key.modifiers.intersects(KeyModifiers::ALT | KeyModifiers::CONTROL);
        match key.code {
            // Alt/Ctrl+arrows; macOS terminals send Option+arrows as Alt+b / Alt+f.
            KeyCode::Left if word => self.cursor = self.word_start(),
            KeyCode::Right if word => self.cursor = self.next_word(),
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => self.cursor = self.word_start(),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => self.cursor = self.next_word(),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value.clear();
                self.cursor = 0;
            }
            KeyCode::Char(c) if !word => {
                let at = self.byte_pos(self.cursor);
                self.value.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte_pos(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Delete if self.cursor < len => {
                let at = self.byte_pos(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            KeyCode::Backspace | KeyCode::Delete => {}
            _ => return false,
        }
        true
    }

    /// Start of the word before the cursor (or of the one it is in).
    fn word_start(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let mut i = self.cursor;
        while i > 0 && chars[i - 1] == ' ' {
            i -= 1;
        }
        while i > 0 && chars[i - 1] != ' ' {
            i -= 1;
        }
        i
    }

    /// Start of the next word.
    fn next_word(&self) -> usize {
        let chars: Vec<char> = self.value.chars().collect();
        let mut i = self.cursor;
        while i < chars.len() && chars[i] != ' ' {
            i += 1;
        }
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        i
    }

    /// Treats each space-separated word as a line (the hosts field shows one name per row) and
    /// moves the cursor to the same column of the previous or next one. False on the first/last.
    pub fn move_line(&mut self, up: bool) -> bool {
        let chars: Vec<char> = self.value.chars().collect();
        let starts: Vec<usize> = std::iter::once(0)
            .chain((0..chars.len()).filter(|&i| chars[i] == ' ').map(|i| i + 1))
            .collect();
        let row = starts.iter().rposition(|&s| s <= self.cursor).unwrap_or(0);
        let target = match (up, row) {
            (true, 0) => return false,
            (true, _) => row - 1,
            (false, _) if row + 1 == starts.len() => return false,
            (false, _) => row + 1,
        };
        let end = starts.get(target + 1).map_or(chars.len(), |&s| s - 1);
        self.cursor = (starts[target] + self.cursor - starts[row]).min(end);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_multibyte_text() {
        let mut input = TextInput::new("кіт");
        input.handle(key(KeyCode::Left));
        input.handle(key(KeyCode::Backspace));
        input.handle(key(KeyCode::Char('и')));
        assert_eq!(input.value, "кит");
        input.handle(key(KeyCode::Home));
        input.handle(key(KeyCode::Delete));
        assert_eq!((input.value.as_str(), input.cursor), ("ит", 0));
    }

    #[test]
    fn jumps_by_word() {
        let mut input = TextInput::new("a.local bb.local  c");
        input.handle(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
        assert_eq!(input.cursor, 18);
        input.handle(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT));
        assert_eq!(input.cursor, 8);
        input.handle(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(input.cursor, 0);
        input.handle(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
        assert_eq!(input.cursor, 8);
        assert_eq!(input.value, "a.local bb.local  c");
    }

    #[test]
    fn moves_between_words_as_lines() {
        let mut input = TextInput::new("abc.local x long.local");
        input.cursor = 3; // abc|.local
        assert!(!input.move_line(true));
        assert!(input.move_line(false));
        assert_eq!(input.cursor, 11); // x| (clamped to the end of the short word)
        assert!(input.move_line(false));
        assert_eq!(input.cursor, 13); // l|ong.local
        assert!(!input.move_line(false));
        assert!(input.move_line(true) && input.move_line(true));
        assert_eq!(input.cursor, 1);
    }
}
