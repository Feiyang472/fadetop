use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListState, StatefulWidget, Widget},
};

use crate::state::{AppState, Focus};

pub struct LocalVariableSelection {}

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
                        return StatefulWidget::render(
                            List::new(locals.iter().map(|local_var| {
                                format!(
                                    "{}\n  {}",
                                    local_var.name.clone(),
                                    local_var.repr.clone().unwrap_or_default()
                                )
                            }))
                            .block(self.get_block(
                                &record.frame_key.name.to_string(),
                                state.focus == Focus::LogView,
                            )),
                            area,
                            buf,
                            &mut ListState::default()
                                .with_selected(Some(state.viewport_bound.selected_depth as usize)),
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
