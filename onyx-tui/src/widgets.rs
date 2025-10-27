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
}

impl<'a> InputWidget<'a> {
    pub fn new(input: &'a str, theme: &'a Theme, focused: bool) -> Self {
        Self { input, theme, focused }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let style = if self.focused { self.theme.input_active } else { self.theme.input_inactive };

        let border_style = if self.focused { self.theme.border_focused } else { self.theme.border };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(" Input ", self.theme.title))
            .title_bottom(Span::styled(" Enter to send • Ctrl+C to quit ", self.theme.help_text));

        let input_display = if self.input.is_empty() && !self.focused {
            Span::styled("Type your message...", self.theme.help_text)
        } else {
            Span::styled(self.input, style)
        };

        let paragraph =
            Paragraph::new(Line::from(input_display)).block(block).wrap(Wrap { trim: false });

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
            Line::from(vec![
                Span::styled("Commands: ", self.theme.help_text.add_modifier(Modifier::BOLD)),
                Span::styled("/config ", self.theme.success),
                Span::styled("• ", self.theme.help_text),
                Span::styled("/help ", self.theme.success),
            ]),
            Line::from(vec![
                Span::styled("Scroll: ", self.theme.help_text.add_modifier(Modifier::BOLD)),
                Span::styled("↑↓ ", self.theme.success),
                Span::styled("or ", self.theme.help_text),
                Span::styled("PgUp/PgDn ", self.theme.success),
                Span::styled("• ", self.theme.help_text),
                Span::styled("Home/End ", self.theme.success),
                Span::styled("to jump", self.theme.help_text),
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
