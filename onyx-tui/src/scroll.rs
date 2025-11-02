use ratatui::widgets::ScrollbarState;

const SCROLL_PAGE_AMOUNT: usize = 10;

pub struct ScrollManager {
    position: usize,
    scrollbar_state: ScrollbarState,
    auto_scroll: bool,
}

impl ScrollManager {
    pub fn new() -> Self {
        Self { position: 0, scrollbar_state: ScrollbarState::default(), auto_scroll: true }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn scrollbar_state_mut(&mut self) -> &mut ScrollbarState {
        &mut self.scrollbar_state
    }

    pub fn enable_auto_scroll(&mut self) {
        self.auto_scroll = true;
    }

    pub fn scroll_to_top(&mut self) {
        self.position = 0;
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.position = self.position.saturating_sub(amount);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.position = self.position.saturating_add(amount);
        self.auto_scroll = false;
    }

    pub fn scroll_page_up(&mut self) {
        self.scroll_up(SCROLL_PAGE_AMOUNT);
    }

    pub fn scroll_page_down(&mut self) {
        self.scroll_down(SCROLL_PAGE_AMOUNT);
    }

    pub fn update(&mut self, content_length: usize, viewport_height: usize) {
        self.position = if self.auto_scroll {
            content_length.saturating_sub(viewport_height)
        } else {
            self.position.min(content_length.saturating_sub(1))
        };

        self.scrollbar_state =
            self.scrollbar_state.content_length(content_length).position(self.position);
    }

    pub fn ensure_visible(&mut self, line: usize, viewport_height: usize, content_length: usize) {
        if line < self.position {
            self.position = line;
        } else if line >= self.position + viewport_height {
            self.position = line.saturating_sub(viewport_height - 1);
        }

        let max_scroll = content_length.saturating_sub(viewport_height);
        self.position = self.position.min(max_scroll);
    }

    pub fn reset(&mut self) {
        self.position = 0;
        self.auto_scroll = true;
    }
}

impl Default for ScrollManager {
    fn default() -> Self {
        Self::new()
    }
}
