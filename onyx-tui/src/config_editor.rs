use onyx_core::{Config, ConfigSchema, FieldDescriptor, FieldType, FieldValue};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation},
};

use crate::scroll::ScrollManager;
use crate::text_input::TextInputState;
use crate::theme::Theme;
use crate::widgets::ConfigFieldWidget;

pub struct ConfigEditor {
    pub config: Config,
    fields: Vec<FieldDescriptor>,
    sections: Vec<String>,
    selected_index: usize,
    pub editing: bool,
    input_state: TextInputState,
    pub show_enum_menu: bool,
    pub enum_menu_selected: usize,
    scroll_manager: ScrollManager,
}

impl ConfigEditor {
    pub fn new(config: Config) -> Self {
        let sections = Config::sections();
        let fields = Config::fields();

        Self {
            config,
            sections,
            fields,
            selected_index: 0,
            editing: false,
            input_state: TextInputState::new(),
            show_enum_menu: false,
            enum_menu_selected: 0,
            scroll_manager: ScrollManager::new(),
        }
    }

    fn current_field(&self) -> &FieldDescriptor {
        &self.fields[self.selected_index]
    }

    fn current_value(&self) -> String {
        let field = self.current_field();
        if field.is_group {
            return String::new();
        }

        field.get_value(&self.config).map(|v| v.as_display_string()).unwrap_or_default()
    }

    fn set_current_value(&mut self, value: String) {
        let field_id = self.current_field().id.clone();
        let field_type = self.current_field().field_type;
        let is_group = self.current_field().is_group;

        if is_group {
            return;
        }

        let field_value = FieldValue::from_string(value, field_type);
        let _ = self.config.set_field(&field_id, field_value);
    }

    pub fn start_editing(&mut self) {
        let is_group = self.current_field().is_group;
        let field_type = self.current_field().field_type;
        let enum_values = self.current_field().enum_values.clone();

        if is_group {
            return;
        }

        self.editing = true;
        let value = self.current_value();
        self.input_state = TextInputState::with_text(value.clone());

        if field_type == FieldType::Enum {
            self.show_enum_menu = true;
            self.enum_menu_selected = enum_values
                .iter()
                .position(|v| v.to_lowercase() == value.to_lowercase())
                .unwrap_or(0);
        }
    }

    pub fn cancel_editing(&mut self) {
        self.editing = false;
        self.input_state.clear();
        self.show_enum_menu = false;
    }

    pub fn save_current_field(&mut self) {
        let field = self.current_field();

        if field.field_type == FieldType::Enum {
            if self.enum_menu_selected < field.enum_values.len() {
                let selected_value = field.enum_values[self.enum_menu_selected].clone();
                self.set_current_value(selected_value);
            }
        } else {
            self.set_current_value(self.input_state.text().to_string());
        }

        self.cancel_editing();
    }

    pub fn insert_char(&mut self, c: char) {
        if self.show_enum_menu {
            return;
        }
        self.input_state.insert_char(c);
    }

    pub fn delete_char(&mut self) {
        if self.show_enum_menu {
            return;
        }
        self.input_state.delete_char_before();
    }

    pub fn delete_char_forward(&mut self) {
        if self.show_enum_menu {
            return;
        }
        self.input_state.delete_char_after();
    }

    pub fn move_cursor_left(&mut self) {
        self.input_state.move_cursor_left(false);
    }

    pub fn move_cursor_right(&mut self) {
        self.input_state.move_cursor_right(false);
    }

    pub fn next_field(&mut self) {
        if self.selected_index < self.fields.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn prev_field(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn enum_menu_up(&mut self) {
        if self.enum_menu_selected > 0 {
            self.enum_menu_selected -= 1;
        }
    }

    pub fn enum_menu_down(&mut self) {
        let field = self.current_field();
        let enum_count = field.enum_values.len();
        if self.enum_menu_selected < enum_count.saturating_sub(1) {
            self.enum_menu_selected += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_manager.scroll_up(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_manager.scroll_down(1);
    }

    pub fn scroll_page_up(&mut self) {
        self.scroll_manager.scroll_page_up();
    }

    pub fn scroll_page_down(&mut self) {
        self.scroll_manager.scroll_page_down();
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_manager.scroll_to_top();
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        terminal_cursor: &crate::cursor::TerminalCursor,
    ) {
        let dialog_width = area.width.min(90);
        let dialog_height = area.height.min(30);

        let dialog_area = Rect {
            x: (area.width.saturating_sub(dialog_width)) / 2,
            y: (area.height.saturating_sub(dialog_height)) / 2,
            width: dialog_width,
            height: dialog_height,
        };

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_focused)
            .title(Span::styled(" Configuration Editor ", theme.title))
            .title_alignment(Alignment::Center);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(inner);

        self.render_fields(frame, chunks[0], theme, terminal_cursor);
        self.render_footer(frame, chunks[1], theme);

        if self.show_enum_menu {
            self.render_enum_menu(frame, dialog_area, theme);
        }
    }

    fn render_fields(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        terminal_cursor: &crate::cursor::TerminalCursor,
    ) {
        let mut lines = Vec::new();
        let mut selected_line: usize = 0;
        let mut current_line: usize = 0;
        let mut cursor_position: Option<(u16, u16)> = None;

        for section in &self.sections {
            if !lines.is_empty() {
                lines.push(Line::from(""));
                current_line += 1;
            }
            lines.push(Line::from(Span::styled(
                format!("═══ {} ═══", section),
                theme.title.add_modifier(Modifier::BOLD),
            )));
            current_line += 1;
            lines.push(Line::from(""));
            current_line += 1;

            for field in &self.fields {
                if &field.section == section {
                    let field_index =
                        self.fields.iter().position(|f| f.id == field.id).unwrap_or(0);
                    let is_selected = field_index == self.selected_index;
                    let is_editing = is_selected && self.editing && !self.show_enum_menu;

                    if is_selected {
                        selected_line = current_line;
                    }

                    let display_value = if is_editing {
                        self.input_state.text().to_string()
                    } else {
                        self.get_display_value(field)
                    };

                    let widget = ConfigFieldWidget::new(
                        field.label.clone(),
                        display_value,
                        is_selected,
                        is_editing,
                        self.input_state.cursor_position(),
                        theme,
                    );

                    lines.push(widget.render());

                    if is_editing {
                        let line_in_viewport =
                            current_line.saturating_sub(self.scroll_manager.position());
                        cursor_position =
                            widget.get_cursor_position(area, area.y + line_in_viewport as u16);
                    }

                    current_line += 1;
                }
            }
        }

        let content_length = lines.len();
        let viewport_height = area.height as usize;

        self.scroll_manager.ensure_visible(selected_line, viewport_height, content_length);
        self.scroll_manager.update(content_length, viewport_height);

        let paragraph = Paragraph::new(lines).scroll((self.scroll_manager.position() as u16, 0));
        frame.render_widget(paragraph, area);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            self.scroll_manager.scrollbar_state_mut(),
        );

        if let Some((x, y)) = cursor_position
            && terminal_cursor.is_visible()
        {
            frame.set_cursor_position((x, y));
        }
    }

    fn get_display_value(&self, field: &FieldDescriptor) -> String {
        let value = field
            .get_value(&self.config)
            .ok()
            .map(|v| match &v {
                FieldValue::Enum(s) => field
                    .enum_values
                    .iter()
                    .find(|ev| ev.to_lowercase() == s.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| s.clone()),
                FieldValue::OptionalString(Some(s)) if field.id.contains("api_key") => {
                    Self::mask_api_key(s)
                }
                FieldValue::OptionalString(Some(s)) => s.clone(),
                FieldValue::OptionalString(None) => String::new(),
                FieldValue::String(s) => s.clone(),
                FieldValue::U64(n) => n.to_string(),
            })
            .unwrap_or_default();

        if value.is_empty() { "(empty)".to_string() } else { value }
    }

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            return String::new();
        }
        let len = key.len();
        if len <= 8 { "*".repeat(len) } else { format!("{}...{}", &key[..4], &key[len - 4..]) }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let hints = if self.editing {
            "[Enter] Save  [Esc] Cancel  [←/→] Move cursor"
        } else {
            "[↑/↓] Scroll  [Tab/Shift+Tab] Navigate fields  [Enter] Edit  [Ctrl+S] Save  [Esc] Close"
        };

        let footer = Paragraph::new(Line::from(Span::styled(hints, theme.help_text)))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP).border_style(theme.border));

        frame.render_widget(footer, area);
    }

    fn render_enum_menu(&self, frame: &mut Frame, parent_area: Rect, theme: &Theme) {
        let field = self.current_field();
        let enum_values = &field.enum_values;
        let menu_height = enum_values.len() as u16 + 2;
        let menu_width = 30;

        let menu_area = Rect {
            x: (parent_area.width.saturating_sub(menu_width)) / 2,
            y: (parent_area.height.saturating_sub(menu_height)) / 2,
            width: menu_width,
            height: menu_height,
        };

        frame.render_widget(Clear, menu_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_focused)
            .title(Span::styled(format!(" Select {} ", field.label), theme.title));

        let inner = block.inner(menu_area);
        frame.render_widget(block, menu_area);

        let mut lines = Vec::new();
        for (i, value) in enum_values.iter().enumerate() {
            let style = if i == self.enum_menu_selected {
                theme.input_active.add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == self.enum_menu_selected { "▶ " } else { "  " };
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, value), style)));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
