use crate::config::AppConfig;

use crate::errors::AppError;
use crate::priority::SpiedRecordQueueMap;
use crate::{state::AppState, tabs::terminal_event::UpdateEvent};
use anyhow::Error;
use py_spy::sampler;
use ratatui::{DefaultTerminal, crossterm};

use std::env;
use std::sync::RwLock;
use std::time::Duration;
use std::{sync::Arc, thread};

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

pub trait SamplerOps: Send + 'static {
    fn push_to_queue(self, record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>)
    -> Result<(), Error>;
}

impl SamplerOps for sampler::Sampler {
    fn push_to_queue(
        self,
        record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    ) -> Result<(), Error> {
        for sample in self {
            for trace in sample.traces.iter() {
                record_queue_map
                    .write()
                    .map_err(|_| AppError::SamplerSenderError)?
                    .increment(trace);
            }
        }

        Ok(())
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

    pub async fn run<S: SamplerOps>(
        mut self,
        terminal: DefaultTerminal,
        sampler: S,
    ) -> Result<(), Error> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<UpdateEvent>(2);

        self.run_event_senders(event_tx, sampler)?;

        self.app_state
            .run_until_error(terminal, &mut event_rx)
            .await?;
        Ok(())
    }
}
