use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::time::SystemTime;

use crate::theme::Theme;
use onyx_core::{Message, Role};

pub struct MessageWidget<'a> {
    message: &'a Message,
    theme: &'a Theme,
    width: usize,
    timestamp_format: &'a str,
}

impl<'a> MessageWidget<'a> {
    pub fn new(
        message: &'a Message,
        theme: &'a Theme,
        width: usize,
        timestamp_format: &'a str,
    ) -> Self {
        Self { message, theme, width, timestamp_format }
    }

    pub fn render(&self) -> Vec<Line<'a>> {
        let (prefix, style) = match self.message.role {
            Role::User => ("You", self.theme.user_message),
            Role::Assistant => ("Onyx", self.theme.assistant_message),
        };

        let mut lines = Vec::new();

        let timestamp = self.format_timestamp(self.message.timestamp);
        lines.push(Line::from(vec![
            Span::styled("┌─ ", self.theme.border),
            Span::styled(prefix, style),
            Span::styled(" ", self.theme.border),
            Span::styled(timestamp, self.theme.help_text),
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

    fn format_timestamp(&self, timestamp: SystemTime) -> String {
        use chrono::{DateTime, Utc};
        let datetime: DateTime<Utc> = timestamp.into();
        datetime.format(self.timestamp_format).to_string()
    }
}

pub struct InputWidget<'a> {
    input: &'a str,
    theme: &'a Theme,
    focused: bool,
    is_processing: bool,
    spinner_state: usize,
    cursor_position: usize,
    selection_range: Option<(usize, usize)>,
}

impl<'a> InputWidget<'a> {
    pub fn new(
        input: &'a str,
        theme: &'a Theme,
        focused: bool,
        is_processing: bool,
        spinner_state: usize,
        cursor_position: usize,
        selection_range: Option<(usize, usize)>,
    ) -> Self {
        Self {
            input,
            theme,
            focused,
            is_processing,
            spinner_state,
            cursor_position,
            selection_range,
        }
    }

    fn get_spinner_char(&self) -> &'static str {
        const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        SPINNER_CHARS[self.spinner_state % SPINNER_CHARS.len()]
    }

    fn render_input_with_cursor(&self, base_style: Style) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let selection_style = self.theme.input_active.add_modifier(Modifier::REVERSED);

        if let Some((sel_start, sel_end)) = self.selection_range {
            if sel_start > 0 {
                let before_sel = &self.input[..sel_start];
                spans.extend(self.style_input_text(before_sel, base_style));
            }

            let actual_end = sel_end.min(self.input.len());
            if sel_start < actual_end {
                let selected = &self.input[sel_start..actual_end];
                spans.push(Span::styled(selected.to_string(), selection_style));
            }

            if actual_end < self.input.len() {
                let after_sel = &self.input[actual_end..];
                spans.extend(self.style_input_text(after_sel, base_style));
            }
        } else if self.cursor_position >= self.input.len() {
            spans.extend(self.style_input_text(self.input, base_style));
            if self.focused {
                spans.push(Span::styled("█".to_string(), self.theme.input_active));
            }
        } else {
            let before_cursor = &self.input[..self.cursor_position];
            spans.extend(self.style_input_text(before_cursor, base_style));

            if self.focused {
                let char_at_cursor = self.input.chars().nth(self.cursor_position).unwrap_or(' ');
                spans.push(Span::styled(
                    char_at_cursor.to_string(),
                    self.theme.input_active.add_modifier(Modifier::REVERSED),
                ));
            }

            let after_cursor_start = self.cursor_position
                + self.input[self.cursor_position..].chars().next().map_or(1, |c| c.len_utf8());
            if after_cursor_start < self.input.len() {
                let after_cursor = &self.input[after_cursor_start..];
                spans.extend(self.style_input_text(after_cursor, base_style));
            }
        }

        spans
    }

    fn style_input_text(&self, text: &str, base_style: Style) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '/' {
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), base_style));
                    current.clear();
                }

                let mut command = String::from('/');
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() {
                    command.push(chars[i]);
                    i += 1;
                }

                spans.push(Span::styled(command, self.theme.success.add_modifier(Modifier::BOLD)));
            } else {
                current.push(chars[i]);
                i += 1;
            }
        }

        if !current.is_empty() {
            spans.push(Span::styled(current, base_style));
        }

        if spans.is_empty() && !text.is_empty() {
            spans.push(Span::styled(text.to_string(), base_style));
        }

        spans
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
                Span::styled("[Ctrl+H] ", self.theme.success),
                Span::styled("history ", self.theme.help_text),
                Span::styled("• ", self.theme.border),
                Span::styled("[Ctrl+L] ", self.theme.success),
                Span::styled("clear ", self.theme.help_text),
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
            self.render_input_with_cursor(style)
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

pub struct CommandMenuWidget<'a> {
    commands: &'a [(&'a str, &'a str)],
    selected: usize,
    theme: &'a Theme,
}

impl<'a> CommandMenuWidget<'a> {
    pub fn new(commands: &'a [(&'a str, &'a str)], selected: usize, theme: &'a Theme) -> Self {
        Self { commands, selected, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_focused)
            .title(Span::styled(" Commands ", self.theme.title));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();
        for (idx, (cmd, desc)) in self.commands.iter().enumerate() {
            let line = if idx == self.selected {
                Line::from(vec![
                    Span::styled(" ▶ ", self.theme.success.add_modifier(Modifier::BOLD)),
                    Span::styled(*cmd, self.theme.success.add_modifier(Modifier::BOLD)),
                    Span::styled(" - ", self.theme.help_text),
                    Span::styled(*desc, self.theme.help_text.add_modifier(Modifier::ITALIC)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("   ", self.theme.help_text),
                    Span::styled(*cmd, self.theme.success),
                    Span::styled(" - ", self.theme.help_text),
                    Span::styled(*desc, self.theme.help_text),
                ])
            };
            lines.push(line);
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner_area);
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            result.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0;

        for word in paragraph.split_whitespace() {
            let word_len = word.len();

            if current_width + word_len + 1 > width && !current_line.is_empty() {
                result.push(current_line.clone());
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
                        result.push(current_line.clone());
                        current_line.clear();
                        current_width = 0;
                    }
                    result.push(chunk_str.to_string());
                }
            } else {
                current_line.push_str(word);
                current_width += word_len;
            }
        }

        if !current_line.is_empty() {
            result.push(current_line);
        }
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}
