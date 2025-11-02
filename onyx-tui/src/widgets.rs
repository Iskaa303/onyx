use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::time::SystemTime;

use crate::cursor::{CursorPosition, InlineCursor};
use crate::theme::Theme;
use onyx_core::{CursorStyle, Message, Role};

pub struct MessageWidget<'a> {
    message: &'a Message,
    theme: &'a Theme,
    width: usize,
    timestamp_format: &'a str,
    cursor_style: CursorStyle,
}

impl<'a> MessageWidget<'a> {
    pub fn new(
        message: &'a Message,
        theme: &'a Theme,
        width: usize,
        timestamp_format: &'a str,
        cursor_style: CursorStyle,
    ) -> Self {
        Self { message, theme, width, timestamp_format, cursor_style }
    }

    pub fn render(&self) -> Vec<Line<'a>> {
        let (prefix, style) = match self.message.role {
            Role::User => ("You", self.theme.user_message),
            Role::Assistant => ("Onyx", self.theme.assistant_message),
        };

        let mut lines = Vec::new();

        let timestamp = self.format_timestamp(self.message.timestamp);
        let mut title_spans = vec![
            Span::styled("┌─ ", self.theme.border),
            Span::styled(prefix, style),
            Span::styled(" ", self.theme.border),
            Span::styled(timestamp, self.theme.help_text),
        ];

        if self.message.is_streaming {
            title_spans.push(Span::styled(" ", self.theme.border));
            title_spans.push(Span::styled("⠿", self.theme.success.add_modifier(Modifier::BOLD)));
            title_spans.push(Span::styled(" streaming", self.theme.help_text));
        }

        title_spans.push(Span::styled(" ─", self.theme.border));
        lines.push(Line::from(title_spans));

        let content_width = self.width.saturating_sub(4);

        if let Some(thinking) = &self.message.thinking {
            lines.push(Line::from(vec![
                Span::styled("│ ", self.theme.border),
                Span::styled("💭 Thinking...", self.theme.help_text.add_modifier(Modifier::ITALIC)),
            ]));

            let thinking_style = self.theme.help_text.add_modifier(Modifier::DIM);
            let wrapped_thinking = wrap_text(thinking, content_width.saturating_sub(2));

            for line in wrapped_thinking {
                lines.push(Line::from(vec![
                    Span::styled("│   ", self.theme.border),
                    Span::styled(line, thinking_style),
                ]));
            }

            lines.push(Line::from(vec![Span::styled("│", self.theme.border)]));
        }

        if !self.message.content.is_empty() || self.message.is_streaming {
            let wrapped_lines = wrap_text(&self.message.content, content_width);

            if wrapped_lines.is_empty() && self.message.is_streaming {
                let inline_cursor = InlineCursor::new(self.cursor_style);
                lines.push(Line::from(vec![
                    Span::styled("│ ", self.theme.border),
                    inline_cursor.render_char(style),
                ]));
            } else {
                for (idx, line) in wrapped_lines.iter().enumerate() {
                    let mut line_spans = vec![Span::styled("│ ", self.theme.border)];

                    if idx == wrapped_lines.len() - 1 && self.message.is_streaming {
                        line_spans.push(Span::styled(
                            line.clone(),
                            style.remove_modifier(Modifier::BOLD),
                        ));

                        let inline_cursor = InlineCursor::new(self.cursor_style);
                        line_spans.push(inline_cursor.render_char(style));
                    } else {
                        line_spans.push(Span::styled(
                            line.clone(),
                            style.remove_modifier(Modifier::BOLD),
                        ));
                    }

                    lines.push(Line::from(line_spans));
                }
            }
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
        } else {
            spans.extend(self.style_input_text(self.input, base_style));
        }

        spans
    }

    pub fn get_cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.focused {
            return None;
        }

        let pos = CursorPosition::calculate(self.input, self.cursor_position, area, true)?;
        Some((pos.x, pos.y))
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

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        terminal_cursor: &crate::cursor::TerminalCursor,
    ) {
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

        let input_text = if self.input.is_empty() && !self.focused {
            vec![Span::styled("Type your message here...", self.theme.help_text)]
        } else {
            self.render_input_with_cursor(style)
        };

        let paragraph =
            Paragraph::new(Line::from(input_text)).block(block).wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        if self.focused
            && let Some((x, y)) = self.get_cursor_position(area)
            && terminal_cursor.is_visible()
        {
            frame.set_cursor_position((x, y));
        }
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

pub struct ConfigFieldWidget<'a> {
    label: String,
    value: String,
    is_selected: bool,
    is_editing: bool,
    cursor_position: usize,
    theme: &'a Theme,
}

impl<'a> ConfigFieldWidget<'a> {
    pub fn new(
        label: String,
        value: String,
        is_selected: bool,
        is_editing: bool,
        cursor_position: usize,
        theme: &'a Theme,
    ) -> Self {
        Self { label, value, is_selected, is_editing, cursor_position, theme }
    }

    pub fn render(&self) -> Line<'static> {
        let label_style = if self.is_selected {
            self.theme.input_active.add_modifier(Modifier::BOLD)
        } else {
            self.theme.help_text
        };

        let value_style = if self.is_editing {
            self.theme.input_active
        } else if self.is_selected {
            self.theme.border_focused
        } else {
            Style::default()
        };

        let prefix = if self.is_selected { "▶ " } else { "  " };
        let label_width = 22;
        let formatted_label = format!("{}{:<width$}", prefix, self.label, width = label_width);

        Line::from(vec![
            Span::styled(formatted_label, label_style),
            Span::raw(" : "),
            Span::styled(self.value.clone(), value_style),
        ])
    }

    pub fn get_cursor_position(&self, area: Rect, line_y: u16) -> Option<(u16, u16)> {
        if !self.is_editing {
            return None;
        }

        const PREFIX_WIDTH: usize = 2;
        const LABEL_WIDTH: usize = 22;
        const SEPARATOR_WIDTH: usize = 3;

        let cursor_x =
            area.x + (PREFIX_WIDTH + LABEL_WIDTH + SEPARATOR_WIDTH + self.cursor_position) as u16;
        let cursor_y = line_y;

        Some((cursor_x, cursor_y))
    }
}
