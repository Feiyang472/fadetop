use py_spy::stack_trace::LocalVariable;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::priority::SpiedRecordQueue;

use super::Blocked;

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

pub struct LocalVariableWidget<'a> {
    fqn: Option<String>,
    locals: Option<&'a Vec<LocalVariable>>,
}

impl<'a> LocalVariableWidget<'a> {
    pub fn blocked(self, focused: bool) -> Blocked<'a, LocalVariableWidget<'a>> {
        let block = Block::default()
            .title(Line::from("Live Stack").bold().left_aligned())
            .borders(Borders::TOP | Borders::LEFT)
            .border_style(if focused {
                Style::new().blue().on_white().bold().italic()
            } else {
                Style::default()
            });
        Blocked { sub: self, block }
    }

    pub fn from_queue(queue: Option<&'a SpiedRecordQueue>, selected_depth: usize) -> Self {
        if let Some(record) = queue.and_then(|q| q.unfinished_events.get(selected_depth)) {
            Self {
                fqn: Some(record.frame_key.fqn()),
                locals: record.locals(),
            }
        } else {
            Self {
                fqn: None,
                locals: None,
            }
        }
    }
}

impl StatefulWidget for LocalVariableWidget<'_> {
    type State = LocalVariableSelection;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let local_section = if let Some(fqn) = self.fqn {
            let [fqn_section, local_section] = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length((fqn.len() as u16).div_ceil(area.width)),
                    Constraint::Fill(1),
                ])
                .areas(area);
            Widget::render(
                Paragraph::new(fqn.clone())
                    .style(Style::new().fg(Color::White).bg(Color::Blue))
                    .wrap(Wrap { trim: true }),
                fqn_section,
                buf,
            );
            local_section
        } else {
            area
        };

        if let Some(locals) = self.locals {
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
                .scroll(state.scroll_offset)
                .wrap(Wrap { trim: true }),
                local_section,
                buf,
            );
        }
    }
}
