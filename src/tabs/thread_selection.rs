use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};
use remoteprocess::Tid;

use crate::{
    priority::{SpiedRecordQueue, SpiedRecordQueueMap},
    state::{AppState, Focus},
};

pub struct ThreadSelectionWidget {}

#[derive(Debug, Clone, Default)]
pub(crate) struct ThreadSelectionState {
    selected_thread: usize,
    pub available_threads: Vec<Tid>,
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
    fn num_threads(&self) -> usize {
        self.available_threads.len()
    }

    fn next_thread(&mut self) {
        self.selected_thread = self
            .selected_thread
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads())
            .unwrap_or(0);
    }

    fn prev_thread(&mut self) {
        let num_threads = self.num_threads();
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

    pub(crate) fn nlines(&self, width: u16) -> u16 {
        (12 * self.num_threads() as u16).div_ceil(width) + 1
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let mut x = area.left();
        let mut n_row = area.top();
        let titles_length = self.num_threads();
        for (i, tid) in self.available_threads.iter().enumerate() {
            let last_title = titles_length - 1 == i;
            let remaining_width = area.right().saturating_sub(x);

            if remaining_width <= 12 {
                x = area.left();
                n_row += 1;
            }

            let pos = buf.set_line(x, n_row, &Line::from("["), remaining_width);
            x = pos.0;
            let remaining_width = area.right().saturating_sub(x);
            if remaining_width == 0 {
                break;
            }

            let pos = buf.set_line(
                x,
                n_row,
                &Line::from(format!("{:08x}", tid)),
                remaining_width,
            );
            if i == self.selected_thread {
                buf.set_style(
                    Rect {
                        x,
                        y: n_row,
                        width: pos.0.saturating_sub(x),
                        height: 1,
                    },
                    (Color::default(), Color::Blue),
                );
            }
            x = pos.0;
            let remaining_width = area.right().saturating_sub(x);
            if remaining_width == 0 {
                break;
            }

            let pos = buf.set_line(x, n_row, &Line::from("]"), remaining_width);
            x = pos.0;
            let remaining_width = area.right().saturating_sub(x);
            if remaining_width == 0 || last_title {
                break;
            }

            let pos = buf.set_span(x, n_row, &Span::from(", "), remaining_width);
            x = pos.0;
        }
    }
}

impl StatefulWidget for ThreadSelectionWidget {
    type State = AppState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = self.get_block(state.focus == Focus::ThreadList);
        let inner = block.inner(area);
        block.render(area, buf);
        state.thread_selection.render_tabs(inner, buf);
    }
}
