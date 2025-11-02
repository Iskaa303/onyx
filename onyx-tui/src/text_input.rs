use std::time::Instant;

const UNDO_GROUP_INTERVAL_MS: u128 = 500;
const MAX_UNDO_HISTORY: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputState {
    text: String,
    cursor_position: usize,
    selection_start: Option<usize>,
}

impl TextInputState {
    pub fn new() -> Self {
        Self { text: String::new(), cursor_position: 0, selection_start: None }
    }

    pub fn with_text(text: String) -> Self {
        let cursor_position = text.len();
        Self { text, cursor_position, selection_start: None }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_start.map(|start| {
            if start < self.cursor_position {
                (start, self.cursor_position)
            } else {
                (self.cursor_position, start)
            }
        })
    }

    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    pub fn take_text(&mut self) -> String {
        self.cursor_position = 0;
        self.clear_selection();
        std::mem::take(&mut self.text)
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, &c.to_string());
            self.cursor_position = start + 1;
            self.clear_selection();
        } else {
            self.text.insert(self.cursor_position, c);
            self.cursor_position += 1;
        }
    }

    pub fn delete_char_before(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, "");
            self.cursor_position = start;
            self.clear_selection();
        } else if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.text.remove(self.cursor_position);
        }
    }

    pub fn delete_char_after(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, "");
            self.cursor_position = start;
            self.clear_selection();
        } else if self.cursor_position < self.text.len() {
            self.text.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self, with_selection: bool) {
        if with_selection {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_position);
            }
            if self.cursor_position > 0 {
                self.cursor_position -= 1;
            }
        } else if self.has_selection() {
            if let Some((start, _)) = self.selection_range() {
                self.cursor_position = start;
            }
            self.clear_selection();
        } else if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self, with_selection: bool) {
        if with_selection {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_position);
            }
            if self.cursor_position < self.text.len() {
                self.cursor_position += 1;
            }
        } else if self.has_selection() {
            if let Some((_, end)) = self.selection_range() {
                self.cursor_position = end;
            }
            self.clear_selection();
        } else if self.cursor_position < self.text.len() {
            self.cursor_position += 1;
        }
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_position = self.text.len();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
        self.clear_selection();
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
        self.text.replace_range(start..end, replacement);
        self.cursor_position = start + replacement.len();
        self.clear_selection();
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UndoManager {
    history: Vec<TextInputState>,
    position: usize,
    last_save_time: Instant,
}

impl UndoManager {
    pub fn new() -> Self {
        Self { history: vec![TextInputState::new()], position: 0, last_save_time: Instant::now() }
    }

    pub fn save(&mut self, state: &TextInputState, force: bool) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_save_time);
        let should_save = force || elapsed.as_millis() > UNDO_GROUP_INTERVAL_MS;

        if should_save {
            if self.position < self.history.len() {
                self.history.truncate(self.position + 1);
            }

            if self.history.last() != Some(state) {
                self.history.push(state.clone());
                self.position = self.history.len() - 1;

                if self.history.len() > MAX_UNDO_HISTORY {
                    self.history.remove(0);
                    self.position = self.position.saturating_sub(1);
                }
            }

            self.last_save_time = now;
        }
    }

    pub fn undo(&mut self) -> Option<TextInputState> {
        if self.position > 0 {
            self.position -= 1;
            Some(self.history[self.position].clone())
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.history = vec![TextInputState::new()];
        self.position = 0;
        self.last_save_time = Instant::now();
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}
