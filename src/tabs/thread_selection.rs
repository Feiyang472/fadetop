use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};
use remoteprocess::Tid;

use crate::priority::{SpiedRecordQueue, SpiedRecordQueueMap, ThreadInfo};

use super::Blocked;

#[derive(Debug, Clone, Default)]
pub struct ThreadSelectionState {
    selected_thread_index: usize,
    available_threads: Vec<ThreadInfo>,
}

pub struct ThreadSelectionWidget {}

impl ThreadSelectionWidget {
    pub fn blocked<'b>(self, focused: bool) -> Blocked<'b, ThreadSelectionWidget> {
        Blocked {
            sub: self,
            block: Block::new()
                .borders(Borders::TOP)
                .title("Threads")
                .border_style(if focused {
                    Style::new().blue().on_white().bold().italic()
                } else {
                    Style::default()
                }),
        }
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

        let titles_length = self.num_threads();
        self.selected_thread_index = self
            .selected_thread_index
            .min(titles_length.saturating_sub(1));

        let mut spans = Vec::new();

        let mut last_pid = None;

        for (i, tinfo) in self.available_threads.iter().enumerate() {
            if spans.len() > 0 {
                spans.push(Span::from(" "));
            }
            if Some(tinfo.pid) != last_pid {
                spans.push(Span::from(format!("{:08x}❯", tinfo.pid)).bg(Color::Green));
            }
            spans.push(Span::styled(
                match tinfo.name {
                    Some(ref name) => format!("[{}]", name),
                    None => format!("[{:08x}]", tinfo.tid),
                },
                if i == self.selected_thread_index {
                    Style::default().bg(Color::default()).fg(Color::Blue).bold()
                } else {
                    Style::default()
                },
            ));
            last_pid = Some(tinfo.pid);
        }

        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    pub fn update_threads(&mut self, qmaps: &SpiedRecordQueueMap) {
        self.available_threads
            .retain(|tinfo| qmaps.contains_key(&tinfo.tid));
        let mut sorted_qmaps: Vec<(&Tid, &SpiedRecordQueue)> = qmaps.iter().collect();
        sorted_qmaps.sort_by(|(_, q1), (_, q2)| q1.thread_info.pid.cmp(&q2.thread_info.pid));
        for (tid, q) in sorted_qmaps {
            if let None = self
                .available_threads
                .iter()
                .find(|tinfo| tinfo.tid == *tid)
            {
                self.available_threads.push(q.thread_info.clone());
            }
        }
    }
}

impl StatefulWidget for ThreadSelectionWidget {
    type State = ThreadSelectionState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render_tabs(area, buf);
    }
}
