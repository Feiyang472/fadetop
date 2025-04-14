use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, StatefulWidget, Tabs, Widget},
};

use crate::state::AppState;

pub struct TabSelectionWidget {}

impl StatefulWidget for TabSelectionWidget {
    type State = AppState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let highlight_style = (Color::default(), Color::Blue);

        let mut quit = false;

        match state.forgetting_queues.read() {
            Ok(queues) => Tabs::new(queues.keys().map(|tid| format!("{:#x}", tid)))
                .block(Block::new().borders(Borders::TOP).title("Threads"))
                .highlight_style(highlight_style)
                .select(state.selected_tab)
                .padding("[", "]")
                .divider(", ")
                .render(area, buf),
            Err(_err) => {
                quit = true;
            }
        };

        if quit {
            state.quit();
        }
    }
}
