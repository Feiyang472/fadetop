use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};
use remoteprocess::{Pid, Tid};

use crate::priority::{SpiedRecordQueue, SpiedRecordQueueMap, ThreadInfo};

use super::{StatefulWidgetExt, get_scroll};

#[derive(Debug, Clone)]
pub struct ThreadSelectionState {
    selected_thread_index: (usize, usize),
    available_threads: Vec<(Pid, Vec<ThreadInfo>)>,
    show_processes: bool,
}

impl Default for ThreadSelectionState {
    fn default() -> Self {
        Self {
            selected_thread_index: (0, 0),
            available_threads: Vec::new(),
            show_processes: true,
        }
    }
}

pub struct ThreadSelectionWidget {
    pub(crate) focused: bool,
}

impl ThreadSelectionState {
    pub fn handle_focused_event(&mut self, key: &KeyEvent) {
        let (pi, ti) = &mut self.selected_thread_index;
        match key.code {
            event::KeyCode::Right => *ti = ti.saturating_add(1),
            event::KeyCode::Left => *ti = ti.saturating_sub(1),
            event::KeyCode::Down => *pi = pi.saturating_add(1),
            event::KeyCode::Up => *pi = pi.saturating_sub(1),
            event::KeyCode::Char('p') => {
                self.show_processes ^= true;
            }
            _ => {}
        }
    }

    fn get_selected_pt(&self) -> (Option<Pid>, Option<Tid>) {
        let (pi, ti) = self.selected_thread_index;
        if let Some((pid, tinfos)) = self.available_threads.get(pi) {
            (
                Some(*pid),
                tinfos.get(ti).map(|t| t.tid),
            )
        } else {
            (None, None)
        }
    }

    pub fn select_thread<'a>(
        &self,
        queues: &'a SpiedRecordQueueMap,
    ) -> Option<&'a SpiedRecordQueue> {
        queues.get(&self.get_selected_pt().1?)
    }

    pub fn update_threads(&mut self, qmaps: &SpiedRecordQueueMap) {
        let (maybe_pid, maybe_tid) = self.get_selected_pt();
        self.available_threads = qmaps
            .iter()
            .map(|(_, q)| q.thread_info.clone())
            .into_group_map_by(|info| info.pid)
            .into_iter()
            .sorted_by(|(pid1, _), (pid2, _)| pid1.cmp(pid2))
            .collect();
        
        let (pi, ti) = &mut self.selected_thread_index;

        *pi = if let Some(new_pindex) = maybe_pid.and_then(|pid_orig| {
            self.available_threads
                .iter()
                .position(|(pid, _)| *pid == pid_orig)
        }) {
            let ts = &self.available_threads[new_pindex].1;
            *ti = if let Some(new_tindex) =
                maybe_tid.and_then(|tid_orig| ts.iter().position(|tinfo| tinfo.tid == tid_orig))
            {
                new_tindex
            } else {
                *ti % ts.len().max(1)
            };
            new_pindex
        } else {
            *ti = 0;
            *pi % self.available_threads.len().max(1)
        };
    }
}

impl StatefulWidget for ThreadSelectionWidget {
    type State = ThreadSelectionState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }

        let threads_tab = if state.available_threads.len() > 1 && state.show_processes {
            let [processes_tab, threads_tab] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Length(13), Constraint::Fill(1)])
                .spacing(1)
                .areas(area);

            let mut process_lines = Vec::new();

            for (i, (pid, tinfos)) in state.available_threads.iter().enumerate() {
                if i == state.selected_thread_index.0 {
                    process_lines.push(
                        Line::from(format!("{:08x}({:})❯", pid, tinfos.len())).bg(Color::Blue),
                    );
                } else {
                    process_lines.push(
                        Line::from(format!("{:08x}({:})", pid, tinfos.len())).bg(Color::Green),
                    );
                }
            }
            Paragraph::new(process_lines)
                .scroll((
                    get_scroll(state.selected_thread_index.0 as u16, area.height),
                    0,
                ))
                .render(processes_tab, buf);
            if self.focused {
                buf.cell_mut((
                    area.left() - 1,
                    area.top() + (state.selected_thread_index.0 as u16) % area.height.max(1),
                ))
                .map(|cell| cell.set_char('↕'));
            }

            threads_tab
        } else {
            area
        };

        let thread_lines = state
            .available_threads
            .get(state.selected_thread_index.0)
            .map_or_else(
                || Vec::new(),
                |(_, thread_infos)| {
                    thread_infos
                        .iter()
                        .enumerate()
                        .map(|(j, tinfo)| {
                            let mut style = Style::default();
                            let mut padding = ('[', ']');
                            if j == state.selected_thread_index.1 {
                                style = style.bg(Color::default()).fg(Color::Blue).bold();
                                if self.focused {
                                    padding = ('←', '→');
                                }
                            }
                            Line::styled(
                                match tinfo.name {
                                    Some(ref name) => format!("{}{}{}", padding.0, name, padding.1),
                                    None => format!("{}{:08x}{}", padding.0, tinfo.tid, padding.1),
                                },
                                style,
                            )
                        })
                        .collect()
                },
            );

        Paragraph::new(thread_lines)
            .block(Block::new())
            .scroll((
                get_scroll(state.selected_thread_index.1 as u16, area.height),
                0,
            ))
            .render(threads_tab, buf);
    }
}

impl StatefulWidgetExt for ThreadSelectionWidget {
    fn get_block(&self, state: &mut Self::State) -> Block {
        let mut block = Block::default()
            .title("Threads")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.focused {
                Style::new().blue().on_dark_gray().bold().italic()
            } else {
                Style::default()
            });

        if self.focused {
            block = block.title_bottom(
                Line::from(vec![Span::from("p").underlined(), "rocesses".into()]).right_aligned(),
            )
        }
        if state.show_processes {
            block
        } else {
            block.title_bottom(
                Line::from(
                    state
                        .available_threads
                        .get(state.selected_thread_index.0)
                        .and_then(|(pid, _)| format!("{:08x}", pid).into())
                        .unwrap_or_default(),
                )
                .left_aligned(),
            )
        }
    }
}
