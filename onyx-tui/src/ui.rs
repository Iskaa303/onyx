use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use thiserror::Error;

use crate::config_editor::ConfigEditor;
use crate::theme::Theme;
use crate::widgets::{HelpWidget, InputWidget, MessageWidget};
use onyx_core::{Config, ConfigSchema, Message};

#[derive(Debug, Error)]
pub enum UiError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, UiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Chat,
    Config,
}

pub struct App {
    messages: Vec<Message>,
    input: String,
    cursor_position: usize,
    selection_start: Option<usize>,
    should_quit: bool,
    show_help: bool,
    submit: bool,
    scroll: usize,
    scroll_state: ScrollbarState,
    theme: Theme,
    input_focused: bool,
    auto_scroll: bool,
    is_processing: bool,
    spinner_state: usize,
    show_command_menu: bool,
    command_menu_selected: usize,
    available_commands: Vec<(&'static str, &'static str)>,
    undo_history: Vec<(String, usize)>,
    undo_position: usize,
    undo_group_timer: std::time::Instant,
    config: Config,
    mode: AppMode,
    config_editor: Option<ConfigEditor>,
    config_saved: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            selection_start: None,
            should_quit: false,
            show_help: true,
            submit: false,
            scroll: 0,
            scroll_state: ScrollbarState::default(),
            theme: Theme::default(),
            input_focused: true,
            auto_scroll: true,
            is_processing: false,
            spinner_state: 0,
            show_command_menu: false,
            command_menu_selected: 0,
            available_commands: vec![
                ("/help", "Show help information"),
                ("/config", "Open configuration editor"),
                ("/now", "Insert current date and time"),
                ("/save", "Save conversation to log file"),
            ],
            undo_history: vec![(String::new(), 0)],
            undo_position: 0,
            undo_group_timer: std::time::Instant::now(),
            config,
            mode: AppMode::Chat,
            config_editor: None,
            config_saved: false,
        }
    }

    pub fn open_config_editor(&mut self) {
        self.config_editor = Some(ConfigEditor::new(self.config.clone()));
        self.mode = AppMode::Config;
    }

    pub fn close_config_editor(&mut self) {
        self.config_editor = None;
        self.mode = AppMode::Chat;
        self.config_saved = false;
    }

    pub fn save_config_from_editor(&mut self) -> Result<()> {
        if let Some(editor) = &self.config_editor {
            self.config = editor.config.clone();
            self.config
                .save()
                .map_err(|e| UiError::IoError(std::io::Error::other(e.to_string())))?;
            self.config_saved = true;
        }
        Ok(())
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.auto_scroll = true;
    }

    pub fn take_input(&mut self) -> Option<String> {
        if !self.submit {
            return None;
        }
        self.submit = false;
        if self.input.is_empty() {
            return None;
        }

        let input = std::mem::take(&mut self.input);

        self.cursor_position = 0;
        self.selection_start = None;
        self.show_command_menu = false;
        self.command_menu_selected = 0;
        self.undo_history = vec![(String::new(), 0)];
        self.undo_position = 0;

        Some(Self::expand_now_command(&input))
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn set_processing(&mut self, processing: bool) {
        self.is_processing = processing;
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_state = self.spinner_state.wrapping_add(1);
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll = 0;
        self.auto_scroll = true;
    }

    pub fn save_conversation_log(&self) -> Result<String> {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let filename = format!("onyx-conversation-{}.log", timestamp);

        let mut log_content = String::new();
        log_content.push_str("Onyx Conversation Log\n");
        log_content
            .push_str(&format!("Generated: {}\n", self.config.format_timestamp(SystemTime::now())));
        log_content.push_str(&format!("{}\n\n", "=".repeat(80)));

        for msg in &self.messages {
            let role = match msg.role {
                onyx_core::Role::User => "USER",
                onyx_core::Role::Assistant => "ASSISTANT",
            };
            let timestamp = self.config.format_timestamp(msg.timestamp);
            log_content.push_str(&format!("[{}] {} at {}\n", role, role, timestamp));
            log_content.push_str(&format!("{}\n", "-".repeat(80)));
            log_content.push_str(&msg.content);
            log_content.push_str(&format!("\n\n{}\n\n", "=".repeat(80)));
        }

        fs::write(&filename, log_content)?;
        Ok(filename)
    }

    fn update_command_menu(&mut self) {
        let input_before_cursor = &self.input[..self.cursor_position];

        if let Some(last_word_start) = input_before_cursor.rfind(|c: char| c.is_whitespace()) {
            let word = &input_before_cursor[last_word_start + 1..];
            if word.starts_with('/') {
                self.show_command_menu = true;
                return;
            }
        } else if input_before_cursor.starts_with('/') {
            self.show_command_menu = true;
            return;
        }

        self.show_command_menu = false;
        self.command_menu_selected = 0;
    }

    fn get_filtered_commands(&self) -> Vec<(&'static str, &'static str)> {
        let input_before_cursor = &self.input[..self.cursor_position];

        let command_prefix =
            if let Some(last_word_start) = input_before_cursor.rfind(|c: char| c.is_whitespace()) {
                &input_before_cursor[last_word_start + 1..]
            } else {
                input_before_cursor
            };

        if !command_prefix.starts_with('/') {
            return Vec::new();
        }

        self.available_commands
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(command_prefix))
            .copied()
            .collect()
    }

    pub fn get_command_menu_state(&self) -> Option<(Vec<(&'static str, &'static str)>, usize)> {
        if self.show_command_menu {
            let filtered = self.get_filtered_commands();
            if !filtered.is_empty() {
                return Some((filtered, self.command_menu_selected));
            }
        }
        None
    }

    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        if let Some(start) = self.selection_start {
            let (sel_start, sel_end) = if start < self.cursor_position {
                (start, self.cursor_position)
            } else {
                (self.cursor_position, start)
            };
            Some((sel_start, sel_end))
        } else {
            None
        }
    }

    fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    fn save_to_undo(&mut self, force: bool) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.undo_group_timer);
        let should_save = force || elapsed.as_millis() > 500;

        if should_save {
            if self.undo_position < self.undo_history.len() {
                self.undo_history.truncate(self.undo_position + 1);
            }

            let state = (self.input.clone(), self.cursor_position);
            if self.undo_history.last() != Some(&state) {
                self.undo_history.push(state);
                self.undo_position = self.undo_history.len() - 1;

                if self.undo_history.len() > 100 {
                    self.undo_history.remove(0);
                    self.undo_position = self.undo_position.saturating_sub(1);
                }
            }

            self.undo_group_timer = now;
        }
    }

    fn undo(&mut self) {
        if self.undo_position > 0 {
            self.undo_position -= 1;
            let (input, cursor) = self.undo_history[self.undo_position].clone();
            self.input = input;
            self.cursor_position = cursor;
            self.clear_selection();
            self.update_command_menu();
        }
    }

    fn expand_now_command(input: &str) -> String {
        let now = chrono::Local::now();
        let formatted = now.format("%Y-%m-%d %H:%M:%S").to_string();
        input.replace("/now", &formatted)
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        match self.mode {
            AppMode::Chat => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(frame.area());

                self.render_chat_area(frame, chunks[0]);

                let input_widget = InputWidget::new(
                    &self.input,
                    &self.theme,
                    self.input_focused,
                    self.is_processing,
                    self.spinner_state,
                    self.cursor_position,
                    self.get_selection_range(),
                );
                input_widget.render(frame, chunks[1]);

                if let Some((commands, selected)) = self.get_command_menu_state() {
                    self.render_command_menu(frame, chunks[1], &commands, selected);
                }
            }
            AppMode::Config => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(frame.area());

                self.render_chat_area(frame, chunks[0]);

                let input_widget = InputWidget::new(
                    &self.input,
                    &self.theme,
                    false,
                    self.is_processing,
                    self.spinner_state,
                    self.cursor_position,
                    None,
                );
                input_widget.render(frame, chunks[1]);

                if let Some(editor) = &mut self.config_editor {
                    editor.render(frame, frame.area(), &self.theme);
                }

                if self.config_saved {
                    self.render_save_notification(frame, frame.area());
                }
            }
        }
    }

    fn render_save_notification(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::Clear;

        let width = 40;
        let height = 5;
        let notification_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, notification_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.success)
            .title(Span::styled(" Success ", self.theme.success));

        let inner = block.inner(notification_area);
        frame.render_widget(block, notification_area);

        let message = Paragraph::new(Line::from(vec![
            Span::styled("✓ ", self.theme.success),
            Span::raw("Configuration saved!"),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(message, inner);
    }

    fn render_command_menu(
        &self,
        frame: &mut Frame,
        input_area: Rect,
        commands: &[(&str, &str)],
        selected: usize,
    ) {
        use crate::widgets::CommandMenuWidget;

        let menu_height = (commands.len() as u16).min(5) + 2;
        let menu_width = 50.min(input_area.width.saturating_sub(4));

        let menu_area = Rect {
            x: input_area.x + 2,
            y: input_area.y.saturating_sub(menu_height),
            width: menu_width,
            height: menu_height,
        };

        let menu_widget = CommandMenuWidget::new(commands, selected, &self.theme);
        menu_widget.render(frame, menu_area);
    }

    fn render_chat_area(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border)
            .title(Span::styled(" Onyx Chat ", self.theme.title))
            .title_alignment(Alignment::Center);

        let inner_area = block.inner(area);
        let chat_width = inner_area.width.saturating_sub(2) as usize;

        let mut lines = Vec::new();

        if self.show_help {
            lines.extend(HelpWidget::new(&self.theme).render());
        }

        for msg in &self.messages {
            let message_widget =
                MessageWidget::new(msg, &self.theme, chat_width, &self.config.timestamp_format);
            lines.extend(message_widget.render());
            lines.push(Line::from(""));
        }

        let content_length = lines.len();
        let viewport_height = inner_area.height as usize;

        self.scroll = if self.auto_scroll {
            content_length.saturating_sub(viewport_height)
        } else {
            self.scroll.min(content_length.saturating_sub(1))
        };

        self.scroll_state = self.scroll_state.content_length(content_length).position(self.scroll);

        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(lines).scroll((self.scroll as u16, 0)), inner_area);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            inner_area,
            &mut self.scroll_state,
        );
    }

    pub fn handle_event(&mut self) -> Result<bool> {
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }

            if self.mode == AppMode::Config {
                return self.handle_config_event(key);
            }

            match key.code {
                KeyCode::Char('c')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.should_quit = true;
                    return Ok(true);
                }
                KeyCode::Char('l')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.clear_chat();
                    return Ok(true);
                }
                KeyCode::Char('a')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.selection_start = Some(0);
                    self.cursor_position = self.input.len();
                    return Ok(true);
                }
                KeyCode::Char('z')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.undo();
                    return Ok(true);
                }
                KeyCode::Char('d')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    if self.input.is_empty() {
                        self.should_quit = true;
                    } else {
                        self.save_to_undo(true);
                        self.input.clear();
                        self.cursor_position = 0;
                        self.clear_selection();
                        self.update_command_menu();
                    }
                    return Ok(true);
                }
                KeyCode::Up => {
                    if self.show_command_menu {
                        let filtered = self.get_filtered_commands();
                        if !filtered.is_empty() {
                            self.command_menu_selected =
                                self.command_menu_selected.saturating_sub(1);
                        }
                    } else {
                        self.scroll = self.scroll.saturating_sub(1);
                        self.auto_scroll = false;
                    }
                }
                KeyCode::Down => {
                    if self.show_command_menu {
                        let filtered = self.get_filtered_commands();
                        if !filtered.is_empty() && self.command_menu_selected < filtered.len() - 1 {
                            self.command_menu_selected += 1;
                        }
                    } else {
                        self.scroll = self.scroll.saturating_add(1);
                        self.auto_scroll = false;
                    }
                }
                KeyCode::PageUp => {
                    self.scroll = self.scroll.saturating_sub(10);
                    self.auto_scroll = false;
                }
                KeyCode::PageDown => {
                    self.scroll = self.scroll.saturating_add(10);
                    self.auto_scroll = false;
                }
                KeyCode::Home => {
                    self.scroll = 0;
                    self.auto_scroll = false;
                }
                KeyCode::End => self.auto_scroll = true,
                KeyCode::Char(c) => {
                    let is_word_boundary = c.is_whitespace() || c.is_ascii_punctuation();
                    self.save_to_undo(is_word_boundary);
                    if let Some((start, end)) = self.get_selection_range() {
                        self.input.replace_range(start..end, &c.to_string());
                        self.cursor_position = start + 1;
                        self.clear_selection();
                    } else {
                        self.input.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                    self.update_command_menu();
                    self.show_help = false;
                    return Ok(true);
                }
                KeyCode::Backspace => {
                    self.save_to_undo(true);
                    if let Some((start, end)) = self.get_selection_range() {
                        self.input.replace_range(start..end, "");
                        self.cursor_position = start;
                        self.clear_selection();
                    } else if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.input.remove(self.cursor_position);
                    }
                    self.update_command_menu();
                    return Ok(true);
                }
                KeyCode::Delete => {
                    self.save_to_undo(true);
                    if let Some((start, end)) = self.get_selection_range() {
                        self.input.replace_range(start..end, "");
                        self.cursor_position = start;
                        self.clear_selection();
                    } else if self.cursor_position < self.input.len() {
                        self.input.remove(self.cursor_position);
                    }
                    self.update_command_menu();
                }
                KeyCode::Left => {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_position);
                        }
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                    } else if self.selection_start.is_some() {
                        if let Some((start, _)) = self.get_selection_range() {
                            self.cursor_position = start;
                        }
                        self.clear_selection();
                    } else if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    self.update_command_menu();
                }
                KeyCode::Right => {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_position);
                        }
                        if self.cursor_position < self.input.len() {
                            self.cursor_position += 1;
                        }
                    } else if self.selection_start.is_some() {
                        if let Some((_, end)) = self.get_selection_range() {
                            self.cursor_position = end;
                        }
                        self.clear_selection();
                    } else if self.cursor_position < self.input.len() {
                        self.cursor_position += 1;
                    }
                    self.update_command_menu();
                }
                KeyCode::Tab => {
                    if self.show_command_menu {
                        let filtered = self.get_filtered_commands();
                        if !filtered.is_empty() {
                            self.save_to_undo(true);
                            let selected_idx = self.command_menu_selected % filtered.len();
                            let selected_command = filtered[selected_idx].0;

                            let input_before_cursor = &self.input[..self.cursor_position];
                            let cmd_start = if let Some(pos) =
                                input_before_cursor.rfind(|c: char| c.is_whitespace())
                            {
                                pos + 1
                            } else {
                                0
                            };

                            self.input
                                .replace_range(cmd_start..self.cursor_position, selected_command);
                            self.cursor_position = cmd_start + selected_command.len();
                            self.show_command_menu = false;
                            self.command_menu_selected = 0;
                        }
                        return Ok(true);
                    }
                }
                KeyCode::Enter => {
                    self.show_help = false;
                    self.submit = true;
                    return Ok(true);
                }
                _ => {}
            }
        }

        self.tick_spinner();
        Ok(false)
    }

    pub fn handle_command(&mut self, cmd: &str) -> Option<String> {
        match cmd {
            "/config" => {
                self.open_config_editor();
                None
            }
            "/save" => match self.save_conversation_log() {
                Ok(filename) => Some(format!("Conversation saved to: {}", filename)),
                Err(e) => Some(format!("Failed to save conversation: {}", e)),
            },
            "/help" => Some(
                "Commands:\n  \
                    /config - Open configuration editor\n  \
                    /save - Save conversation to log file\n  \
                    /help - Show this help\n\n\
                    Navigation:\n  \
                    ↑/↓ - Scroll up/down\n  \
                    PgUp/PgDn - Scroll page up/down\n  \
                    Home/End - Jump to top/bottom\n\n\
                    Actions:\n  \
                    Ctrl+L - Clear chat\n  \
                    Ctrl+C - Quit"
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn handle_config_event(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        use crossterm::event::KeyModifiers;

        let Some(editor) = &mut self.config_editor else {
            return Ok(false);
        };

        if editor.editing {
            match key.code {
                KeyCode::Enter => editor.save_current_field(),
                KeyCode::Esc => editor.cancel_editing(),
                KeyCode::Char(c) => editor.insert_char(c),
                KeyCode::Backspace => editor.delete_char(),
                KeyCode::Delete => editor.delete_char_forward(),
                KeyCode::Left => editor.move_cursor_left(),
                KeyCode::Right => editor.move_cursor_right(),
                KeyCode::Up if editor.show_enum_menu => editor.enum_menu_up(),
                KeyCode::Down if editor.show_enum_menu => editor.enum_menu_down(),
                _ => return Ok(false),
            }
        } else {
            match key.code {
                KeyCode::Esc => self.close_config_editor(),
                KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => editor.scroll_up(),
                KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    editor.scroll_down()
                }
                KeyCode::Up => editor.prev_field(),
                KeyCode::Down => editor.next_field(),
                KeyCode::PageUp => editor.scroll_page_up(),
                KeyCode::PageDown => editor.scroll_page_down(),
                KeyCode::Home => editor.scroll_to_top(),
                KeyCode::Enter => editor.start_editing(),
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.save_config_from_editor()?
                }
                _ => return Ok(false),
            }
        }

        Ok(true)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
