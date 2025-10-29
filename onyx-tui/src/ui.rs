use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use thiserror::Error;

use crate::theme::Theme;
use crate::widgets::{HelpWidget, InputWidget, MessageWidget};
use onyx_core::{Config, Message};

#[derive(Debug, Error)]
pub enum UiError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, UiError>;

pub struct App {
    messages: Vec<Message>,
    input: String,
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
            theme: Theme::default(),
            input_focused: true,
            auto_scroll: true,
            is_processing: false,
            spinner_state: 0,
        }
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
        Some(std::mem::take(&mut self.input))
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

    pub fn draw(&mut self, frame: &mut Frame) {
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
        );
        input_widget.render(frame, chunks[1]);
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
            let message_widget = MessageWidget::new(msg, &self.theme, chat_width);
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
                KeyCode::Up => {
                    self.scroll = self.scroll.saturating_sub(1);
                    self.auto_scroll = false;
                }
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => {
                    self.scroll = self.scroll.saturating_sub(10);
                    self.auto_scroll = false;
                }
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                KeyCode::Home => {
                    self.scroll = 0;
                    self.auto_scroll = false;
                }
                KeyCode::End => self.auto_scroll = true,
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

        self.tick_spinner();
        Ok(false)
    }

    pub fn handle_command(&mut self, cmd: &str) -> Option<String> {
        match cmd {
            "/config" => {
                let path = Config::config_path_display();
                Some(format!(
                    "Config location: {}\n\nEdit this file to configure your API keys and settings.",
                    path
                ))
            }
            "/help" => Some(
                "Commands:\n  \
                    /config - Show config file path\n  \
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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
