use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::{
    priority::{SpiedRecordQueue, SpiedRecordQueueMap, ThreadInfo},
    state::{AppState, Focus},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ThreadSelectionState {
    selected_thread_index: usize,
    pub available_threads: Vec<ThreadInfo>,
}

pub struct ThreadSelectionWidget {}

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
        self.selected_thread_index = self
            .selected_thread_index
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads())
            .unwrap_or(0);
    }

    fn prev_thread(&mut self) {
        let num_threads = self.num_threads();
        self.selected_thread_index = self
            .selected_thread_index
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
        queues.get(&self.available_threads.get(self.selected_thread_index)?.tid)
    }

    fn render_tabs(&mut self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let mut x = area.left();
        let mut n_row = area.top();
        let titles_length = self.num_threads();
        self.selected_thread_index = self
            .selected_thread_index
            .min(titles_length.saturating_sub(1));

        for (i, tinfo) in self.available_threads.iter().enumerate() {
            let last_title = titles_length - 1 == i;
            let remaining_width = area.right().saturating_sub(x);

            let default_title = format!("{:08x}", tinfo.tid);

            let title: &str = tinfo.name.as_ref().unwrap_or(&default_title);

            if remaining_width <= title.len() as u16 + 4 {
                x = area.left();
                n_row += 1;
            }

            let pos = buf.set_line(x, n_row, &Line::from("["), remaining_width);
            x = pos.0;
            let remaining_width = area.right().saturating_sub(x);
            if remaining_width == 0 {
                break;
            }

            let pos = buf.set_line(x, n_row, &Line::from(title), remaining_width);
            if i == self.selected_thread_index {
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
