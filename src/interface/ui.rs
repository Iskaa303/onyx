use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::core::{Config, Message};

pub struct App {
    messages: Vec<Message>,
    input: String,
    should_quit: bool,
    show_help: bool,
    submit: bool,
    scroll: usize,
    scroll_state: ScrollbarState,
    total_lines: usize,
    max_scroll: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            should_quit: false,
            show_help: true,
            submit: false,
            scroll: 0,
            scroll_state: ScrollbarState::default(),
            total_lines: 0,
            max_scroll: 0,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.scroll_to_bottom();
    }

    pub fn take_input(&mut self) -> Option<String> {
        if !self.submit {
            return None;
        }
        self.submit = false;
        if self.input.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.input))
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = usize::MAX;
    }

    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in text.split_whitespace() {
            let word_len = word.len();

            if current_width + word_len + 1 > width && !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0;
            }

            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }

            if word_len > width {
                // Word is longer than width, split it
                for chunk in word.as_bytes().chunks(width) {
                    let chunk_str = std::str::from_utf8(chunk).unwrap_or("");
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                        current_width = 0;
                    }
                    lines.push(chunk_str.to_string());
                }
            } else {
                current_line.push_str(word);
                current_width += word_len;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(frame.area());

        let mut lines: Vec<Line> = Vec::new();
        let chat_width = chunks[0].width.saturating_sub(4) as usize; // Account for borders and scrollbar

        if self.show_help {
            lines.push(Line::from("Commands: /config, /help | Scroll: ↑↓ or PgUp/PgDn | Ctrl+C to quit")
                .style(Style::default().fg(Color::DarkGray)));
            lines.push(Line::from(""));
        }

        for msg in &self.messages {
            let (prefix, color) = match msg.role {
                crate::core::Role::User => ("You: ", Color::Green),
                crate::core::Role::Assistant => ("AI: ", Color::Cyan),
            };

            // Wrap the message content
            let wrapped_lines = Self::wrap_text(&format!("{}{}", prefix, msg.content), chat_width);
            for line in wrapped_lines {
                lines.push(Line::from(line).style(Style::default().fg(color)));
            }
            lines.push(Line::from(""));
        }

        self.total_lines = lines.len();
        let visible_height = chunks[0].height.saturating_sub(2) as usize;

        self.max_scroll = self.total_lines.saturating_sub(visible_height);
        if self.scroll > self.max_scroll {
            self.scroll = self.max_scroll;
        }

        let messages_widget = Paragraph::new(lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Chat History"))
            .scroll((self.scroll as u16, 0))
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(messages_widget, chunks[0]);

        self.scroll_state = self.scroll_state
            .content_length(self.total_lines)
            .position(self.scroll);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        frame.render_stateful_widget(
            scrollbar,
            chunks[0],
            &mut self.scroll_state,
        );

        let input_widget = Paragraph::new(self.input.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Input (Enter to send)"))
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(input_widget, chunks[1]);
    }

    pub fn handle_event(&mut self) -> Result<bool> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c')
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.should_quit = true;
                            return Ok(true);
                        }
                        KeyCode::Up => {
                            self.scroll_up();
                            return Ok(true);
                        }
                        KeyCode::Down => {
                            self.scroll_down(self.max_scroll);
                            return Ok(true);
                        }
                        KeyCode::PageUp => {
                            self.scroll = self.scroll.saturating_sub(10);
                            return Ok(true);
                        }
                        KeyCode::PageDown => {
                            self.scroll = self.scroll.saturating_add(10);
                            return Ok(true);
                        }
                        KeyCode::Home => {
                            self.scroll = 0;
                            return Ok(true);
                        }
                        KeyCode::End => {
                            self.scroll_to_bottom();
                            return Ok(true);
                        }
                        KeyCode::Char(c) => {
                            self.input.push(c);
                            self.show_help = false;
                            return Ok(true);
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                            return Ok(true);
                        }
                        KeyCode::Enter => {
                            self.show_help = false;
                            self.submit = true;
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn handle_command(&mut self, cmd: &str) -> Option<String> {
        match cmd {
            "/config" => {
                let path = Config::config_path_display();
                Some(format!("Config location: {}\n\nEdit this file to configure your API keys and settings.", path))
            }
            "/help" => {
                Some("Commands:\n  /config - Show config file path\n  /help - Show this help\n\nNavigation:\n  ↑/↓ - Scroll up/down\n  PgUp/PgDn - Scroll page up/down\n  Home/End - Jump to top/bottom\n  Ctrl+C - Quit".to_string())
            }
            _ => None,
        }
    }
}
