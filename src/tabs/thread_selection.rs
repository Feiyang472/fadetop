use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, StatefulWidget, Tabs, Widget},
};

use crate::{
    priority::{SpiedRecordQueue, SpiedRecordQueueMap},
    state::{AppState, Focus},
};

pub struct ThreadSelectionWidget {}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ThreadSelectionState {
    selected_thread: usize,
    pub num_threads: usize,
}

impl ThreadSelectionWidget {
    fn get_block(&self, focused: bool) -> Block {
        Block::new()
            .borders(Borders::TOP)
            .title("Threads")
            .border_style(if focused {
                Style::new().blue().on_white().bold().italic()
            } else {
                Style::default()
            })
    }
}

impl ThreadSelectionState {
    fn next_thread(&mut self) {
        self.selected_thread = self
            .selected_thread
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads)
            .unwrap_or(0);
    }

    fn prev_thread(&mut self) {
        let num_threads = self.num_threads;
        self.selected_thread = self
            .selected_thread
            .overflowing_add(num_threads.saturating_sub(1))
            .0
            .checked_rem(num_threads)
            .unwrap_or(0);
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) {
        match key.code {
            event::KeyCode::Right => self.next_thread(),
            event::KeyCode::Left => self.prev_thread(),
            _ => {}
        }
    }

    pub fn select_thread<'a>(
        &self,
        queues: &'a SpiedRecordQueueMap,
    ) -> Option<&'a SpiedRecordQueue> {
        queues
            .iter()
            .nth(self.selected_thread)
            .map(|(_, queue)| queue)
    }
}

impl StatefulWidget for ThreadSelectionWidget {
    type State = AppState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let highlight_style = (Color::default(), Color::Blue);

        let mut quit = false;

        match state.record_queue_map.read() {
            Ok(queues) => Tabs::new(queues.keys().map(|tid| format!("{:#x}", tid)))
                .block(self.get_block(state.focus == Focus::ThreadList))
                .highlight_style(highlight_style)
                .select(state.thread_selection.selected_thread)
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
