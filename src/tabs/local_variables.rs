use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
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
    fn get_block(&self, frame_name: &str, focused: bool) -> Block {
        Block::default()
            .title(
                Line::from(format!("Local Variables {}", frame_name))
                    .bold()
                    .left_aligned(),
            )
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
                    if let Some(locals) = record.locals() {
                        return Widget::render(
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
                            .wrap(Wrap { trim: true })
                            .block(self.get_block(
                                &record.frame_key.name.to_string(),
                                state.focus == Focus::LogView,
                            )),
                            area,
                            buf,
                        );
                    }
                }

                self.get_block(Default::default(), state.focus == Focus::LogView)
                    .render(area, buf);
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
