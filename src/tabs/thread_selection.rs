use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};
use remoteprocess::{Pid, Tid};

use crate::priority::{SpiedRecordQueue, SpiedRecordQueueMap, ThreadInfo};

use super::Blocked;

#[derive(Debug, Clone, Default)]
pub struct ThreadSelectionState {
    selected_thread_index: (usize, usize),
    available_threads: Vec<(Pid, Vec<ThreadInfo>)>,
}

pub struct ThreadSelectionWidget {}

impl ThreadSelectionWidget {
    pub fn blocked<'b>(self, focused: bool) -> Blocked<'b, ThreadSelectionWidget> {
        Blocked {
            sub: self,
            block: Block::new()
                .title("Threads")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if focused {
                    Style::new().blue().on_dark_gray().bold().italic()
                } else {
                    Style::default()
                }),
        }
    }
}

impl ThreadSelectionState {
    pub fn handle_key_event(&mut self, key: &KeyEvent) {
        match key.code {
            event::KeyCode::Right => {
                self.selected_thread_index.1 = self.selected_thread_index.1.saturating_add(1)
            }
            event::KeyCode::Left => {
                self.selected_thread_index.1 = self.selected_thread_index.1.saturating_sub(1)
            }
            event::KeyCode::Down => {
                self.selected_thread_index.0 = self.selected_thread_index.0.saturating_add(1)
            }
            event::KeyCode::Up => {
                self.selected_thread_index.0 = self.selected_thread_index.0.saturating_sub(1)
            }
            _ => {}
        }
    }

    pub fn select_thread<'a>(
        &self,
        queues: &'a SpiedRecordQueueMap,
    ) -> Option<&'a SpiedRecordQueue> {
        queues.get(
            &self
                .available_threads
                .get(self.selected_thread_index.0)?
                .1
                .get(self.selected_thread_index.1)?
                .tid,
        )
    }

    fn render_tabs(&mut self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let [processes_tab, threads_tab] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(9), Constraint::Fill(1)])
            .spacing(1)
            .areas(area);

        let mut process_lines = Vec::new();

        for (i, (pid, _)) in self.available_threads.iter().enumerate() {
            if i == self.selected_thread_index.0 {
                process_lines.push(Line::from(format!("{:08x}❯", pid)).bg(Color::Blue));
            } else {
                process_lines.push(Line::from(format!("{:08x}", pid)).bg(Color::Green));
            }
        }

        let thread_lines = self
            .available_threads
            .get(self.selected_thread_index.0)
            .map_or_else(
                || Vec::new(),
                |(_, thread_infos)| {
                    thread_infos
                        .iter()
                        .enumerate()
                        .map(|(j, tinfo)| {
                            Line::styled(
                                match tinfo.name {
                                    Some(ref name) => format!("[{}]", name),
                                    None => format!("[{:08x}]", tinfo.tid),
                                },
                                if j == self.selected_thread_index.1 {
                                    Style::default().bg(Color::default()).fg(Color::Blue).bold()
                                } else {
                                    Style::default()
                                },
                            )
                        })
                        .collect()
                },
            );

        Paragraph::new(process_lines).render(processes_tab, buf);
        Paragraph::new(thread_lines)
            .block(Block::new())
            .render(threads_tab, buf);
    }

    pub fn update_threads(&mut self, qmaps: &SpiedRecordQueueMap) {
        self.available_threads = qmaps
            .iter()
            .map(|(_, q)| q.thread_info.clone())
            .into_group_map_by(|info| info.pid)
            .into_iter()
            .sorted_by(|(pid1, _), (pid2, _)| pid1.cmp(pid2))
            .collect();

        self.selected_thread_index = (
            self.selected_thread_index
                .0
                % self.available_threads.len().max(1),
            self.selected_thread_index.1 % (
                self.available_threads
                    .get(self.selected_thread_index.0)
                    .map_or(1, |(_, threads)| threads.len().max(1))
            ),
        );
    }
}

impl StatefulWidget for ThreadSelectionWidget {
    type State = ThreadSelectionState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render_tabs(area, buf);
    }
}
