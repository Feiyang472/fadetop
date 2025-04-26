use crate::config::AppConfig;

use crate::{
    priority::SamplerOps,
    state::AppState,
    tabs::{
        local_variables::LocalVariableWidget, terminal_event::UpdateEvent,
        thread_selection::ThreadSelectionWidget, timeline::TimelineWidget,
    },
};
use anyhow::Error;
use ratatui::{
    DefaultTerminal, crossterm,
    layout::{Constraint, Direction, Layout},
    prelude::Frame,
    style::{Color, Style},
    text::Line,
    widgets::Widget,
};

use std::env;
use std::time::Duration;
use std::{sync::Arc, thread};

#[derive(Debug, Clone, Copy)]
struct Footer {}

impl Widget for Footer {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Line::from(
            "Press Esc to quit, ←↑↓→ to pan within tab, Tab to switch tabs, i/o to zoom in/out",
        )
        .style(Style::default().bg(Color::Rgb(0, 0, 12)))
        .render(area, buf);
    }
}

impl AppConfig {
    pub fn from_configs() -> Result<Self, Error> {
        let config_file = env::var("FADETOP_CONFIG").unwrap_or("fadetop_config.toml".to_string());

        Ok(config::Config::builder()
            .add_source(config::File::with_name(&config_file).required(false))
            .add_source(config::Environment::with_prefix("FADETOP"))
            .build()?
            .try_deserialize::<AppConfig>()?)
    }
}

#[derive(Debug)]
pub struct FadeTopApp {
    pub app_state: AppState,
    update_period: Duration,
}

fn send_terminal_event(tx: tokio::sync::mpsc::Sender<UpdateEvent>) -> Result<(), Error> {
    loop {
        tx.blocking_send(UpdateEvent::Input(crossterm::event::read()?))?;
    }
}

impl FadeTopApp {
    pub fn new(configs: AppConfig) -> Self {
        let mut app_state = AppState::new();
        app_state
            .record_queue_map
            .write()
            .unwrap()
            .with_rules(configs.rules);

        app_state.viewport_bound.width = configs.window_width;

        Self {
            app_state,
            update_period: configs.update_period,
        }
    }

    fn run_event_senders<S: SamplerOps>(
        &self,
        sender: tokio::sync::mpsc::Sender<UpdateEvent>,
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
        let queue = Arc::clone(&self.app_state.record_queue_map);
        thread::spawn({
            move || {
                sampler.push_to_queue(queue).unwrap();
            }
        });

        let update_period = self.update_period;

        // New async event sender
        let async_sender = sender.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_period);
            loop {
                interval.tick().await;
                if async_sender.send(UpdateEvent::Periodic).await.is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    fn render_full_app(&mut self, frame: &mut Frame) {
        let [tab_selector, tab, footer] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Fill(5),
                Constraint::Length(1),
            ])
            .areas(frame.area());
        let [timeline, locals] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(4), Constraint::Fill(1)])
            .areas(tab);
        frame.render_stateful_widget(ThreadSelectionWidget {}, tab_selector, &mut self.app_state);
        frame.render_stateful_widget(TimelineWidget {}, timeline, &mut self.app_state);
        frame.render_stateful_widget(LocalVariableWidget {}, locals, &mut self.app_state);
        frame.render_widget(Footer {}, footer);
    }

    pub async fn run<S: SamplerOps>(
        mut self,
        mut terminal: DefaultTerminal,
        sampler: S,
    ) -> Result<(), Error> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<UpdateEvent>(2);

        self.run_event_senders(event_tx, sampler)?;

        while self.app_state.is_running() {
            terminal.draw(|frame| self.render_full_app(frame))?;
            match event_rx.recv().await {
                None => {
                    break;
                }
                Some(event) => event.update_state(&mut self)?,
            };
        }
        Ok(())
    }
}
