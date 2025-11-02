use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    Block,
    BlockBlinking,
    Line,
    LineBlinking,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self::BlockBlinking
    }
}

impl fmt::Display for CursorStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block => write!(f, "block"),
            Self::BlockBlinking => write!(f, "block_blinking"),
            Self::Line => write!(f, "line"),
            Self::LineBlinking => write!(f, "line_blinking"),
        }
    }
}

impl FromStr for CursorStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "block" => Ok(Self::Block),
            "block_blinking" => Ok(Self::BlockBlinking),
            "line" => Ok(Self::Line),
            "line_blinking" => Ok(Self::LineBlinking),
            _ => Err(format!("Invalid cursor style: {}", s)),
        }
    }
}

impl CursorStyle {
    pub fn is_blinking(self) -> bool {
        matches!(self, Self::BlockBlinking | Self::LineBlinking)
    }

    pub fn is_line(self) -> bool {
        matches!(self, Self::Line | Self::LineBlinking)
    }

    pub fn char(self) -> &'static str {
        match self {
            Self::Block | Self::BlockBlinking => "█",
            Self::Line | Self::LineBlinking => "│",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(default)]
    pub thinking: Option<String>,
    #[serde(default)]
    pub is_streaming: bool,
    #[serde(default = "SystemTime::now")]
    pub timestamp: SystemTime,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            thinking: None,
            is_streaming: false,
            timestamp: SystemTime::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            thinking: None,
            is_streaming: false,
            timestamp: SystemTime::now(),
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            role: Role::Assistant,
            content: String::new(),
            thinking: None,
            is_streaming: true,
            timestamp: SystemTime::now(),
        }
    }

    pub fn append_content(&mut self, chunk: impl Into<String>) {
        self.content.push_str(&chunk.into());
    }

    pub fn set_thinking(&mut self, thinking: impl Into<String>) {
        self.thinking = Some(thinking.into());
    }

    pub fn append_thinking(&mut self, chunk: impl Into<String>) {
        if let Some(ref mut t) = self.thinking {
            t.push_str(&chunk.into());
        } else {
            self.thinking = Some(chunk.into());
        }
    }

    pub fn finish_streaming(&mut self) {
        self.is_streaming = false;
    }
}
