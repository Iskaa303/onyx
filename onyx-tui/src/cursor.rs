use crossterm::{
    ExecutableCommand,
    cursor::{Hide, SetCursorStyle as CrosstermCursorStyle, Show},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};
use std::io::stdout;
use std::time::Instant;
use thiserror::Error;

use onyx_core::CursorStyle;

#[derive(Debug, Error)]
pub enum CursorError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CursorError>;

pub struct TerminalCursor {
    style: CursorStyle,
    blink_interval_ms: u128,
    visible: bool,
    last_blink_time: Instant,
    last_activity_time: Instant,
    needs_apply: bool,
}

impl TerminalCursor {
    pub fn new(style: CursorStyle, blink_interval_ms: u64) -> Self {
        Self {
            style,
            blink_interval_ms: blink_interval_ms as u128,
            visible: true,
            last_blink_time: Instant::now(),
            last_activity_time: Instant::now(),
            needs_apply: true,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn on_activity(&mut self) {
        self.last_activity_time = Instant::now();
        if !self.visible {
            self.visible = true;
            self.needs_apply = true;
        }
    }

    pub fn update(&mut self) {
        if !self.style.is_blinking() {
            if !self.visible {
                self.visible = true;
                self.needs_apply = true;
            }
            return;
        }

        let now = Instant::now();
        let time_since_activity = now.duration_since(self.last_activity_time).as_millis();

        if time_since_activity < self.blink_interval_ms {
            if !self.visible {
                self.visible = true;
                self.needs_apply = true;
            }
            self.last_blink_time = now;
            return;
        }

        let elapsed = now.duration_since(self.last_blink_time).as_millis();

        if elapsed >= self.blink_interval_ms {
            self.visible = !self.visible;
            self.last_blink_time = now;
            self.needs_apply = true;
        }
    }

    pub fn apply(&mut self) -> Result<()> {
        if !self.needs_apply {
            return Ok(());
        }

        if self.visible {
            let crossterm_style = match self.style {
                CursorStyle::Block | CursorStyle::BlockBlinking => {
                    CrosstermCursorStyle::SteadyBlock
                }
                CursorStyle::Line | CursorStyle::LineBlinking => CrosstermCursorStyle::SteadyBar,
            };
            stdout().execute(Show)?.execute(crossterm_style)?;
        } else {
            stdout().execute(Hide)?;
        }

        self.needs_apply = false;
        Ok(())
    }
}

impl Default for TerminalCursor {
    fn default() -> Self {
        Self::new(CursorStyle::LineBlinking, 500)
    }
}

pub struct CursorPosition {
    pub x: u16,
    pub y: u16,
}

impl CursorPosition {
    pub fn calculate(
        text: &str,
        cursor_index: usize,
        area: Rect,
        has_border: bool,
    ) -> Option<Self> {
        let inner =
            if has_border { Block::default().borders(Borders::ALL).inner(area) } else { area };

        let text_before_cursor = if cursor_index == 0 {
            ""
        } else if cursor_index >= text.len() {
            text
        } else {
            &text[..cursor_index]
        };

        let visual_width = text_before_cursor.chars().count();
        let cursor_x = inner.x + visual_width as u16;
        let cursor_y = inner.y;

        Some(Self { x: cursor_x, y: cursor_y })
    }
}

pub struct InlineCursor {
    style: CursorStyle,
}

impl InlineCursor {
    pub fn new(style: CursorStyle) -> Self {
        Self { style }
    }

    pub fn render_char<'a>(&self, text_style: Style) -> Span<'a> {
        let cursor_char = self.style.char();
        let cursor_style = if self.style.is_blinking() {
            text_style.add_modifier(Modifier::RAPID_BLINK)
        } else {
            text_style
        };

        Span::styled(cursor_char.to_string(), cursor_style)
    }
}

impl Default for InlineCursor {
    fn default() -> Self {
        Self::new(CursorStyle::Block)
    }
}
