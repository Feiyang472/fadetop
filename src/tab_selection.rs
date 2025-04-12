use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Block, StatefulWidget, Tabs, Widget},
};

use crate::state::AppState;

pub struct TabSelectionWidget {}

impl StatefulWidget for TabSelectionWidget {
    type State = AppState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let highlight_style = (Color::default(), Color::Blue);

        Tabs::new(
            state
                .forgetting_queue
                .read()
                .unwrap()
                .unfinished_events
                .keys()
                .map(|tid| format!("{:#x}", tid)),
        )
        .block(Block::bordered().title("Threads"))
        .highlight_style(highlight_style)
        .select(state.selected_tab)
        .padding("[", "]")
        .divider(", ")
        .render(area, buf);
    }
}
