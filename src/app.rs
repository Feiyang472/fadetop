use crate::{
    event::UpdateEvent,
    priority::{ForgetRules, SamplerOps},
    state::AppState,
    thread_selection::ThreadSelectionWidget,
    timeline::{LocalVariableWidget, TimelineWidget},
};
use anyhow::Error;
use ratatui::{
    DefaultTerminal, crossterm,
    layout::{Constraint, Direction, Layout},
    prelude::Frame,
};
use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct FadeTopApp {
    pub app_state: AppState,
}

fn send_terminal_event(tx: mpsc::Sender<UpdateEvent>) -> Result<(), Error> {
    loop {
        tx.send(UpdateEvent::Input(crossterm::event::read()?))?;
    }
}

impl FadeTopApp {
    pub fn new() -> Self {
        Self {
            app_state: AppState::new(),
        }
    }

    pub fn with_rules(self, rules: Vec<ForgetRules>) -> Result<Self, Error> {
        self.app_state
            .forgetting_queues
            .write()
            .map_err(|_| std::sync::PoisonError::new(()))?
            .with_rules(rules);
        Ok(self)
    }

    fn run_event_senders<S: SamplerOps>(
        &self,
        sender: mpsc::Sender<UpdateEvent>,
        sampler: S,
    ) -> Result<(), Error> {
        // Existing terminal event sender
        thread::spawn({
            let cloned_sender = sender.clone();
            move || {
                send_terminal_event(cloned_sender).unwrap();
            }
        });

        // Existing sampler event sender
        let queue = Arc::clone(&self.app_state.forgetting_queues);
        thread::spawn({
            move || {
                sampler.push_to_queue(queue).unwrap();
            }
        });

        // New async event sender
        let async_sender = sender.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));
            loop {
                interval.tick().await;
                if async_sender.send(UpdateEvent::Periodic).is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    fn render_full_app(&mut self, frame: &mut Frame) {
        let [tab_selector, tab] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Max(2), Constraint::Fill(50)])
            .areas(frame.area());
        let [timeline, locals] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(4), Constraint::Fill(1)])
            .areas(tab);
        frame.render_stateful_widget(ThreadSelectionWidget {}, tab_selector, &mut self.app_state);
        frame.render_stateful_widget(TimelineWidget {}, timeline, &mut self.app_state);
        frame.render_stateful_widget(LocalVariableWidget {}, locals, &mut self.app_state);
    }

    pub fn run<S: SamplerOps>(
        mut self,
        mut terminal: DefaultTerminal,
        sampler: S,
    ) -> Result<(), Error> {
        // Initialize a Tokio runtime
        let runtime = tokio::runtime::Runtime::new()?;
        let (event_tx, event_rx) = mpsc::channel::<UpdateEvent>();

        // Run the event senders within the Tokio runtime
        runtime.block_on(async {
            self.run_event_senders(event_tx, sampler)?;
            Ok::<(), Error>(())
        })?;

        while self.app_state.is_running() {
            terminal.draw(|frame| self.render_full_app(frame))?;
            event_rx.recv()?.update_state(&mut self)?;
        }
        Ok(())
    }

    pub fn with_viewport_window(mut self, width: Duration) -> Self {
        self.app_state.viewport_bound.width = width;
        self
    }
}
