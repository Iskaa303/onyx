use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::Theme;
use onyx_core::{Message, Role};

pub struct MessageWidget<'a> {
    message: &'a Message,
    theme: &'a Theme,
    width: usize,
}

impl<'a> MessageWidget<'a> {
    pub fn new(message: &'a Message, theme: &'a Theme, width: usize) -> Self {
        Self { message, theme, width }
    }

    pub fn render(&self) -> Vec<Line<'a>> {
        let (prefix, style) = match self.message.role {
            Role::User => ("You", self.theme.user_message),
            Role::Assistant => ("Onyx", self.theme.assistant_message),
        };

        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("┌─ ", self.theme.border),
            Span::styled(prefix, style),
            Span::styled(" ─", self.theme.border),
        ]));

        let content_width = self.width.saturating_sub(4);
        let wrapped_lines = wrap_text(&self.message.content, content_width);

        for line in wrapped_lines {
            lines.push(Line::from(vec![
                Span::styled("│ ", self.theme.border),
                Span::styled(line, style.remove_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::from(Span::styled("└─", self.theme.border)));

        lines
    }
}

pub struct InputWidget<'a> {
    input: &'a str,
    theme: &'a Theme,
    focused: bool,
    is_processing: bool,
    spinner_state: usize,
}

impl<'a> InputWidget<'a> {
    pub fn new(
        input: &'a str,
        theme: &'a Theme,
        focused: bool,
        is_processing: bool,
        spinner_state: usize,
    ) -> Self {
        Self { input, theme, focused, is_processing, spinner_state }
    }

    fn get_spinner_char(&self) -> &'static str {
        const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        SPINNER_CHARS[self.spinner_state % SPINNER_CHARS.len()]
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let style = if self.focused { self.theme.input_active } else { self.theme.input_inactive };

        let border_style = if self.focused { self.theme.border_focused } else { self.theme.border };

        let title = Line::from(Span::styled(" Input ", self.theme.title));

        let bottom_title = if self.is_processing {
            Line::from(vec![
                Span::styled(" ", self.theme.help_text),
                Span::styled(
                    self.get_spinner_char(),
                    self.theme.success.add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Processing... ", self.theme.help_text),
            ])
        } else {
            Line::from(vec![
                Span::styled(" [Enter] ", self.theme.success),
                Span::styled("send ", self.theme.help_text),
                Span::styled("• ", self.theme.border),
                Span::styled("[Ctrl+L] ", self.theme.success),
                Span::styled("clear ", self.theme.help_text),
                Span::styled("• ", self.theme.border),
                Span::styled("[Ctrl+C] ", self.theme.success),
                Span::styled("quit ", self.theme.help_text),
                Span::styled(" │ ", self.theme.border),
                Span::styled("Tip: ", self.theme.help_text.add_modifier(Modifier::ITALIC)),
                Span::styled("/", self.theme.success.add_modifier(Modifier::BOLD)),
                Span::styled(" for commands", self.theme.help_text.add_modifier(Modifier::ITALIC)),
            ])
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_bottom(bottom_title);

        let input_with_cursor = if self.input.is_empty() {
            if self.focused {
                vec![Span::styled("█", self.theme.input_active)]
            } else {
                vec![Span::styled("Type your message here...", self.theme.help_text)]
            }
        } else {
            vec![
                Span::styled(self.input, style),
                if self.focused {
                    Span::styled("█", self.theme.input_active)
                } else {
                    Span::styled("", self.theme.help_text)
                },
            ]
        };

        let paragraph =
            Paragraph::new(Line::from(input_with_cursor)).block(block).wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

pub struct HelpWidget<'a> {
    theme: &'a Theme,
}

impl<'a> HelpWidget<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    pub fn render(&self) -> Vec<Line<'a>> {
        vec![
            Line::from(vec![Span::styled(
                "Welcome to Onyx! ",
                self.theme.title.add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Quick start: ", self.theme.help_text.add_modifier(Modifier::BOLD)),
                Span::styled("Type your message and press ", self.theme.help_text),
                Span::styled("[Enter]", self.theme.success),
                Span::styled(" to send", self.theme.help_text),
            ]),
            Line::from(vec![
                Span::styled("Commands: ", self.theme.help_text.add_modifier(Modifier::BOLD)),
                Span::styled("/config", self.theme.success),
                Span::styled(" • ", self.theme.help_text),
                Span::styled("/help", self.theme.success),
            ]),
            Line::from(vec![
                Span::styled("Navigation: ", self.theme.help_text.add_modifier(Modifier::BOLD)),
                Span::styled("↑↓", self.theme.success),
                Span::styled(" scroll • ", self.theme.help_text),
                Span::styled("PgUp/PgDn", self.theme.success),
                Span::styled(" page • ", self.theme.help_text),
                Span::styled("Home/End", self.theme.success),
                Span::styled(" jump", self.theme.help_text),
            ]),
            Line::from(""),
        ]
    }
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
