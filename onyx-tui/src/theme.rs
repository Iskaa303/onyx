use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub user_message: Style,
    pub assistant_message: Style,
    pub system_message: Style,
    pub input_active: Style,
    pub input_inactive: Style,
    pub border: Style,
    pub border_focused: Style,
    pub title: Style,
    pub help_text: Style,
    pub error: Style,
    pub success: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl Theme {
    pub fn default_theme() -> Self {
        Self {
            user_message: Style::default()
                .fg(Color::Rgb(138, 180, 248))
                .add_modifier(Modifier::BOLD),
            assistant_message: Style::default().fg(Color::Rgb(166, 227, 161)),
            system_message: Style::default().fg(Color::Rgb(249, 226, 175)),
            input_active: Style::default()
                .fg(Color::Rgb(203, 166, 247))
                .add_modifier(Modifier::BOLD),
            input_inactive: Style::default().fg(Color::Rgb(127, 132, 156)),
            border: Style::default().fg(Color::Rgb(88, 91, 112)),
            border_focused: Style::default()
                .fg(Color::Rgb(203, 166, 247))
                .add_modifier(Modifier::BOLD),
            title: Style::default().fg(Color::Rgb(148, 226, 213)).add_modifier(Modifier::BOLD),
            help_text: Style::default()
                .fg(Color::Rgb(127, 132, 156))
                .add_modifier(Modifier::ITALIC),
            error: Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD),
        }
    }

    pub fn monokai() -> Self {
        Self {
            user_message: Style::default()
                .fg(Color::Rgb(102, 217, 239))
                .add_modifier(Modifier::BOLD),
            assistant_message: Style::default().fg(Color::Rgb(166, 226, 46)),
            system_message: Style::default().fg(Color::Rgb(230, 219, 116)),
            input_active: Style::default()
                .fg(Color::Rgb(249, 38, 114))
                .add_modifier(Modifier::BOLD),
            input_inactive: Style::default().fg(Color::Rgb(117, 113, 94)),
            border: Style::default().fg(Color::Rgb(73, 72, 62)),
            border_focused: Style::default()
                .fg(Color::Rgb(249, 38, 114))
                .add_modifier(Modifier::BOLD),
            title: Style::default().fg(Color::Rgb(174, 129, 255)).add_modifier(Modifier::BOLD),
            help_text: Style::default().fg(Color::Rgb(117, 113, 94)).add_modifier(Modifier::ITALIC),
            error: Style::default().fg(Color::Rgb(249, 38, 114)).add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(166, 226, 46)).add_modifier(Modifier::BOLD),
        }
    }
}
