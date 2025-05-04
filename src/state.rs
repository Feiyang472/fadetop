use std::sync::{Arc, RwLock};

use anyhow::Error;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    priority::SpiedRecordQueueMap,
    tabs::{
        local_variables::{LocalVariableSelection, LocalVariableWidget},
        terminal_event::UpdateEvent,
        thread_selection::{ThreadSelectionState, ThreadSelectionWidget},
        timeline::{TimelineWidget, ViewPortBounds},
    },
};

// Add a Focus enum to track current focus
#[derive(Debug, PartialEq, Eq)]
pub enum Focus {
    ThreadList,
    Timeline,
    LogView,
}

#[derive(Debug)]
pub struct AppState {
    focus: Focus,
    thread_selection: ThreadSelectionState,
    pub(super) viewport_bound: ViewPortBounds,
    local_variable_state: LocalVariableSelection,
    pub record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    running: bool,
}

impl AppState {
    fn quit(&mut self) {
        self.running = false;
    }

    pub async fn run_until_error(
        &mut self,
        mut terminal: DefaultTerminal,
        rx: &mut Receiver<UpdateEvent>,
    ) -> Result<(), Error> {
        while self.running {
            terminal.draw(|frame| self.render_full_app(frame))?;
            match rx.recv().await {
                None => {
                    break;
                }
                Some(event) => event.update_state(self)?,
            };
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            focus: Focus::ThreadList,
            thread_selection: Default::default(),
            record_queue_map: Default::default(),
            viewport_bound: Default::default(),
            local_variable_state: LocalVariableSelection::default(),
            running: true,
        }
    }

    fn render_full_app(&mut self, frame: &mut Frame) {
        let [inner, footer] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(1)])
            .areas(frame.area());
        let [timeline, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(4), Constraint::Fill(1)])
            .areas(inner);
        let [tab_selector, locals] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
            .areas(right);

        match self.record_queue_map.read() {
            Ok(qmaps) => {
                self.thread_selection.update_threads(&qmaps);
                frame.render_stateful_widget(
                    ThreadSelectionWidget {}.blocked(self.focus == Focus::ThreadList),
                    tab_selector,
                    &mut self.thread_selection,
                );
                let queue = self.thread_selection.select_thread(&qmaps);
                frame.render_stateful_widget(
                    TimelineWidget::from_queue(queue)
                        .blocked(self.focus == Focus::Timeline, self.viewport_bound),
                    timeline,
                    &mut self.viewport_bound,
                );
                frame.render_stateful_widget(
                    LocalVariableWidget::from_queue(
                        queue,
                        self.viewport_bound.selected_depth as usize,
                    )
                    .blocked(self.focus == Focus::LogView),
                    locals,
                    &mut self.local_variable_state,
                );
            }
            _ => {
                self.running = false;
            }
        }

        frame.render_widget(
            Line::from(
                "Press Esc to quit, ←↑↓→ to pan within tab, Tab to switch tabs, i/o to zoom in/out",
            )
            .style(Style::default().bg(Color::Rgb(0, 0, 12))),
            footer,
        );
    }

    pub fn handle_crossterm_events(&mut self, term_event: event::Event) -> Result<(), Error> {
        match term_event {
            event::Event::Key(key) => match key.code {
                // Global shortcuts
                event::KeyCode::Esc => Ok(self.quit()),
                event::KeyCode::Tab => {
                    self.focus = match self.focus {
                        Focus::ThreadList => Focus::Timeline,
                        Focus::Timeline => Focus::LogView,
                        Focus::LogView => Focus::ThreadList,
                    };
                    Ok(())
                }
                _ => Ok({
                    match self.focus {
                        Focus::ThreadList => self.thread_selection.handle_key_event(&key),
                        Focus::Timeline => self.viewport_bound.handle_key_event(&key),
                        Focus::LogView => self.local_variable_state.handle_key_event(&key),
                    }
                }),
            },
            _ => Ok(()),
        }
    }
}
