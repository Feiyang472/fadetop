use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::state::{AppState, Focus};

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVariableSelection {
    scroll_offset: (u16, u16),
}

impl LocalVariableSelection {
    fn move_up(&mut self) {
        if self.scroll_offset.0 > 0 {
            self.scroll_offset.0 -= 1;
        }
    }

    fn move_down(&mut self) {
        self.scroll_offset.0 += 1;
    }

    fn move_left(&mut self) {
        if self.scroll_offset.1 > 0 {
            self.scroll_offset.1 -= 1;
        }
    }

    fn move_right(&mut self) {
        self.scroll_offset.1 += 1;
    }

    pub fn reset(&mut self) {
        self.scroll_offset = (0, 0);
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) {
        match key.code {
            event::KeyCode::Up => self.move_up(),
            event::KeyCode::Down => self.move_down(),
            event::KeyCode::Left => self.move_left(),
            event::KeyCode::Right => self.move_right(),
            _ => {}
        }
    }
}

impl LocalVariableWidget {
    fn get_block(&self, focused: bool) -> Block {
        Block::default()
            .title(Line::from("Live Stack").bold().left_aligned())
            .borders(Borders::TOP | Borders::LEFT)
            .border_style(if focused {
                Style::new().blue().on_white().bold().italic()
            } else {
                Style::default()
            })
    }
}

pub struct LocalVariableWidget {}

impl StatefulWidget for LocalVariableWidget {
    type State = AppState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut quit = false;

        match state.record_queue_map.read() {
            Ok(queues) => {
                let queue = state.thread_selection.select_thread(&queues);

                if let Some(record) = queue.and_then(|q| {
                    q.unfinished_events
                        .get(state.viewport_bound.selected_depth as usize)
                }) {
                    let block = self.get_block(state.focus == Focus::LogView);
                    let inner = block.inner(area);
                    block.render(area, buf);

                    let fqn = record.frame_key.fqn();
                    let [fqn_section, local_section] = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![
                            Constraint::Length((fqn.len() as u16).div_ceil(inner.width)),
                            Constraint::Fill(1),
                        ])
                        .areas(inner);

                    Widget::render(
                        Paragraph::new(fqn)
                            .style(Style::new().fg(Color::White).bg(Color::Blue))
                            .wrap(Wrap { trim: true }),
                        fqn_section,
                        buf,
                    );
                    if let Some(locals) = record.locals() {
                        Widget::render(
                            Paragraph::new(
                                locals
                                    .iter()
                                    .flat_map(|local_var| {
                                        vec![
                                            Line::from(local_var.name.clone())
                                                .style(Style::default().fg(Color::Indexed(4))),
                                            Line::from(local_var.repr.clone().unwrap_or_default()),
                                        ]
                                    })
                                    .collect::<Vec<Line>>(),
                            )
                            .scroll(state.local_variable_state.scroll_offset)
                            .wrap(Wrap { trim: true }),
                            local_section,
                            buf,
                        );
                        return;
                    }
                }
            }
            Err(_err) => {
                quit = true;
            }
        };
        if quit {
            state.quit();
        };
    }
}
