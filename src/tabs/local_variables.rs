use py_spy::stack_trace::LocalVariable;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::{Constraint, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, HighlightSpacing, Paragraph, Row, StatefulWidget, Table,
        TableState, Widget, Wrap,
    },
};

use crate::priority::SpiedRecordQueue;

use super::{StatefulWidgetExt, get_scroll};

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVariableSelection {
    scroll_offset: (u16, u16),
    selected: bool,
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

    pub fn handle_focused_event(&mut self, key: &KeyEvent) {
        match key.code {
            event::KeyCode::Up => self.move_up(),
            event::KeyCode::Down => self.move_down(),
            event::KeyCode::Left => self.move_left(),
            event::KeyCode::Right => self.move_right(),
            event::KeyCode::Char('s') => self.selected ^= true,
            _ => {}
        }
    }
}

pub struct LocalVariableWidget<'a> {
    locals: Option<&'a Vec<LocalVariable>>,
    focused: bool,
}

impl<'a> LocalVariableWidget<'a> {
    pub fn focused(self, focused: bool) -> Self {
        Self { focused, ..self }
    }

    pub fn from_queue(queue: Option<&'a SpiedRecordQueue>, selected_depth: usize) -> Self {
        if let Some(record) = queue.and_then(|q| q.unfinished_events.get(selected_depth)) {
            Self {
                locals: record.locals(),
                focused: false,
            }
        } else {
            Self {
                locals: None,
                focused: false,
            }
        }
    }
}

impl StatefulWidget for LocalVariableWidget<'_> {
    type State = LocalVariableSelection;
    fn render(self, local_section: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(locals) = self.locals {
            if state.selected {
                if let Some(variable) = locals.get(state.scroll_offset.0 as usize) {
                    Paragraph::new(vec![
                        Line::from(variable.name.clone()).white().on_blue(),
                        Line::from(variable.repr.clone().unwrap_or_default()),
                    ])
                    .wrap(Wrap { trim: true })
                    .render(local_section, buf);
                };
            } else {
                state.scroll_offset.0 = state.scroll_offset.0 % (locals.len() as u16).max(1);
                StatefulWidget::render(
                    Table::new(
                        locals
                            .iter()
                            .map(|local_var| {
                                Row::new(vec![
                                    local_var.name.clone(),
                                    local_var.repr.clone().unwrap_or_default(),
                                ])
                            })
                            .collect::<Vec<Row>>(),
                        vec![Constraint::Fill(1), Constraint::Fill(3)],
                    )
                    .highlight_spacing(HighlightSpacing::Always)
                    .row_highlight_style(Style::new().fg(Color::LightBlue).bold()),
                    local_section,
                    buf,
                    &mut TableState::default()
                        .with_offset(
                            get_scroll(state.scroll_offset.0, local_section.height) as usize
                        )
                        .with_selected(state.scroll_offset.0 as usize),
                );
                if self.focused {
                    buf.cell_mut((
                        local_section.left() - 1,
                        local_section.top()
                            + (state.scroll_offset.0 as u16) % local_section.height.max(1),
                    ))
                    .map(|cell| cell.set_char('↕'));
                }
            }
        }
    }
}

impl StatefulWidgetExt for LocalVariableWidget<'_> {
    fn get_block(&self, state: &mut Self::State) -> Block {
        let block = Block::default()
            .title(Line::from("Live Stack").bold().left_aligned())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        if self.focused {
            block
                .title_bottom(
                    Line::from(if state.selected {
                        vec!["de".into(), Span::from("s").underlined(), "elect".into()]
                    } else {
                        vec![Span::from("s").underlined(), "elect".into()]
                    })
                    .right_aligned(),
                )
                .border_style(if self.focused {
                    Style::new().blue().on_dark_gray().bold().italic()
                } else {
                    Style::default()
                })
        } else {
            block
        }
    }
}
