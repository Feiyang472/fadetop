use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Error;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use tokio::sync::mpsc::Receiver;

use crate::{
    config::AppConfig,
    priority::SpiedRecordQueueMap,
    tabs::{
        StatefulWidgetExt,
        local_variables::{LocalVariableSelection, LocalVariableWidget},
        terminal_event::UpdateEvent,
        thread_selection::{ThreadSelectionState, ThreadSelectionWidget},
        timeline::{TimelineWidget, ViewPortBounds},
    },
};

/// Focus within the profiling view
#[derive(Debug, PartialEq, Eq)]
pub enum Focus {
    ThreadList,
    Timeline,
    LogView,
}

/// State for the profiling view (when attached to a process)
#[derive(Debug)]
pub struct ProfilingState {
    focus: Focus,
    thread_selection: ThreadSelectionState,
    pub(super) viewport_bound: ViewPortBounds,
    local_variable_state: LocalVariableSelection,
    pub record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    running: bool,
    ratio: u16,
    pub update_period: Duration,
}

impl ProfilingState {
    fn quit(&mut self) {
        self.running = false;
    }

    pub async fn run_until_error(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: &mut Receiver<UpdateEvent>,
    ) -> Result<(), Error> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match rx.recv().await {
                None => break,
                Some(event) => event.update_state(self)?,
            };
        }
        Ok(())
    }

    pub fn new(config: &AppConfig) -> Self {
        let mut record_queue_map: SpiedRecordQueueMap = Default::default();
        record_queue_map.with_rules(config.rules.clone());

        let mut viewport_bound = ViewPortBounds::default();
        viewport_bound.width = config.window_width;

        Self {
            focus: Focus::ThreadList,
            thread_selection: Default::default(),
            viewport_bound,
            local_variable_state: LocalVariableSelection::default(),
            record_queue_map: Arc::new(RwLock::new(record_queue_map)),
            running: true,
            ratio: 80,
            update_period: config.update_period,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let out_block = {
            Block::default()
                .borders(Borders::NONE)
                .title_top(Line::from("Esc").underlined().right_aligned())
                .title_top(Line::from("Tab").underlined().left_aligned())
                .title_top(
                    Line::from(vec![
                        "Zoom ".into(),
                        Span::from("I").underlined(),
                        "n/".into(),
                        Span::from("O").underlined(),
                        "ut".into(),
                    ])
                    .left_aligned(),
                )
                .title_style(Style::default().bg(Color::Rgb(0, 0, 100)))
        };

        let inner = out_block.inner(frame.area());
        let [timeline, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(self.ratio),
                Constraint::Percentage(100 - self.ratio),
            ])
            .areas(inner);
        let [tab_selector, locals] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
            .areas(right);

        match self.record_queue_map.read() {
            Ok(qmaps) => {
                self.thread_selection.update_threads(&qmaps);
                frame.render_stateful_widget(
                    ThreadSelectionWidget {
                        focused: self.focus == Focus::ThreadList,
                    }
                    .blocked(),
                    tab_selector,
                    &mut self.thread_selection,
                );
                let queue = self.thread_selection.select_thread(&qmaps);
                frame.render_stateful_widget(
                    TimelineWidget::from_queue(queue)
                        .focused(self.focus == Focus::Timeline)
                        .blocked(),
                    timeline,
                    &mut self.viewport_bound,
                );
                frame.render_stateful_widget(
                    LocalVariableWidget::from_queue(
                        queue,
                        self.viewport_bound.selected_depth as usize,
                    )
                    .focused(self.focus == Focus::LogView)
                    .blocked(),
                    locals,
                    &mut self.local_variable_state,
                );
            }
            _ => {
                self.running = false;
            }
        }

        frame.render_widget(out_block, frame.area());
    }

    pub fn handle_crossterm_events(&mut self, term_event: event::Event) -> Result<(), Error> {
        match term_event {
            event::Event::Key(key) => match (key.modifiers, key.code) {
                (_, event::KeyCode::Esc | event::KeyCode::Char('q')) => Ok(self.quit()),
                (_, event::KeyCode::Tab) => {
                    self.focus = match self.focus {
                        Focus::ThreadList => Focus::Timeline,
                        Focus::Timeline => Focus::LogView,
                        Focus::LogView => Focus::ThreadList,
                    };
                    Ok(())
                }
                (_, event::KeyCode::Char('i') | event::KeyCode::Char('o')) => {
                    Ok(self.viewport_bound.handle_zoom_event(&key))
                }
                (event::KeyModifiers::CONTROL, event::KeyCode::Right) => {
                    Ok(self.ratio = (self.ratio + 1).min(100))
                }
                (event::KeyModifiers::CONTROL, event::KeyCode::Left) => {
                    Ok(self.ratio = self.ratio.saturating_sub(1))
                }
                _ => Ok({
                    match self.focus {
                        Focus::ThreadList => self.thread_selection.handle_focused_event(&key),
                        Focus::Timeline => self.viewport_bound.handle_focused_event(&key),
                        Focus::LogView => self.local_variable_state.handle_focused_event(&key),
                    }
                }),
            },
            _ => Ok(()),
        }
    }
}
