use crate::{
    event::UpdateEvent, priority::SamplerOps, state::AppState, tab_selection::TabSelectionWidget,
    timeline::TimelineWidget,
};
use anyhow::Error;
use py_spy::Config;
use ratatui::{
    DefaultTerminal, crossterm,
    layout::{Constraint, Direction, Layout},
    prelude::Frame,
};
use remoteprocess::Pid;
use std::{
    sync::{Arc, mpsc},
    thread,
};

pub trait SamplerFactory: Clone + Send + Sync {
    type Sampler: SamplerOps;
    fn create_sampler(&self) -> Result<Self::Sampler, Error>;
}

impl SamplerFactory for (Pid, Config) {
    type Sampler = py_spy::sampler::Sampler;
    fn create_sampler(&self) -> Result<Self::Sampler, Error> {
        Self::Sampler::new(self.0, &self.1)
    }
}

#[derive(Debug)]
pub struct FadeTopApp<F: SamplerFactory> {
    pub app_state: AppState,
    sampler_creater: F,
}

fn send_terminal_event(tx: mpsc::Sender<UpdateEvent>) -> Result<(), Error> {
    loop {
        tx.send(UpdateEvent::Input(crossterm::event::read()?))?;
    }
}

impl<F> FadeTopApp<F>
where
    F: SamplerFactory,
{
    fn run_event_senders(&self, sender: mpsc::Sender<UpdateEvent>) -> Result<(), Error> {
        // Existing terminal event sender
        thread::spawn({
            let cloned_sender = sender.clone();
            move || {
                send_terminal_event(cloned_sender).unwrap();
            }
        });

        // Existing sampler event sender
        let sampler = self.sampler_creater.create_sampler()?;
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

    pub fn new(sampler_creater: F) -> Self {
        Self {
            app_state: AppState::new(),
            sampler_creater,
        }
    }

    fn render_full_app(&mut self, frame: &mut Frame) {
        let [tab_selector, tab] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(50)])
            .areas(frame.area());
        frame.render_stateful_widget(
            TabSelectionWidget {},
            tab_selector,
            &mut self.app_state,
        );
        frame.render_stateful_widget(TimelineWidget {}, tab, &mut self.app_state);
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Error> {
        // Initialize a Tokio runtime
        let runtime = tokio::runtime::Runtime::new()?;
        let (event_tx, event_rx) = mpsc::channel::<UpdateEvent>();

        // Run the event senders within the Tokio runtime
        runtime.block_on(async {
            self.run_event_senders(event_tx)?;
            Ok::<(), Error>(())
        })?;

        while self.app_state.is_running() {
            terminal.draw(|frame| self.render_full_app(frame))?;
            event_rx.recv()?.update_state(&mut self)?;
        }
        Ok(())
    }
}
