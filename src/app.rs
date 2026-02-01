use crate::config::AppConfig;
use crate::priority::SpiedRecordQueueMap;
use crate::processes::ProcessDiscovery;
use crate::state::ProfilingState;
use crate::tabs::process_selection::{ProcessSelectionAction, ProcessSelectionState};
use crate::tabs::terminal_event::UpdateEvent;
use anyhow::Error;
use futures::StreamExt;
use ratatui::{DefaultTerminal, Frame, crossterm};
use remoteprocess::Pid;

use std::env;
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

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

pub trait SamplerOps: Send + 'static + Sized {
    fn push_to_queue(
        self,
        record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn from_config_and_id(config: &AppConfig, pid: Pid) -> Result<Self, Error>;
}

async fn send_terminal_event(tx: tokio::sync::mpsc::Sender<UpdateEvent>) -> Result<(), Error> {
    let mut reader = crossterm::event::EventStream::new();

    loop {
        match reader.next().await {
            Some(event) => {
                tx.send(UpdateEvent::Input(event?)).await?;
            }
            None => {
                continue;
            }
        }
    }
}

impl ProcessSelectionState {
    /// Run the app starting from process selection (landing page mode)
    pub async fn run_with_selection<S: SamplerOps, D: ProcessDiscovery>(
        &mut self,
        configs: &AppConfig,
        mut terminal: DefaultTerminal,
    ) -> Result<(), Error> {
        self.refresh_processes::<D>();

        loop {
            match self.run_selection_loop::<D>(&mut terminal).await? {
                SelectionResult::Quit => return Ok(()),
                SelectionResult::AttachTo(pid) => {
                    match S::from_config_and_id(configs, pid) {
                        Ok(sampler) => {
                            ProfilingState::new(configs)
                                .run_attached(&mut terminal, sampler)
                                .await?
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to attach to {}: {}", pid, e));
                        }
                    }
                    self.refresh_processes::<D>();
                }
            }
        }
    }
    /// Run the process selection UI loop
    async fn run_selection_loop<D: ProcessDiscovery>(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> Result<SelectionResult, Error> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<UpdateEvent>(2);

        // Terminal event sender
        let term_handle = tokio::spawn({
            let tx = event_tx.clone();
            async move {
                let _ = send_terminal_event(tx).await;
            }
        });

        // Periodic refresh
        let update_period = Duration::from_secs(2);
        let periodic_handle = tokio::spawn({
            let tx = event_tx;
            async move {
                let mut interval = tokio::time::interval(update_period);
                loop {
                    interval.tick().await;
                    if tx.send(UpdateEvent::Periodic).await.is_err() {
                        break;
                    }
                }
            }
        });

        let result = loop {
            terminal.draw(|frame| self.render_selection(frame))?;

            match event_rx.recv().await {
                None => break Ok(SelectionResult::Quit),
                Some(UpdateEvent::Input(event)) => {
                    if let crossterm::event::Event::Key(key) = event {
                        match self.handle_focused_event(&key) {
                            ProcessSelectionAction::AttachTo(pid) => {
                                break Ok(SelectionResult::AttachTo(pid));
                            }
                            ProcessSelectionAction::Quit => {
                                break Ok(SelectionResult::Quit);
                            }
                            ProcessSelectionAction::Refresh => {
                                self.refresh_processes::<D>();
                            }
                            ProcessSelectionAction::None => {}
                        }
                    }
                }
                Some(UpdateEvent::Periodic) => {
                    self.refresh_processes::<D>();
                }
                Some(UpdateEvent::Error(e)) => break Err(e.into()),
            }
        };

        term_handle.abort();
        periodic_handle.abort();

        result
    }
    fn render_selection(&mut self, frame: &mut Frame) {
        use crate::tabs::StatefulWidgetExt;
        use crate::tabs::process_selection::ProcessSelectionWidget;
        frame.render_stateful_widget(ProcessSelectionWidget.blocked(), frame.area(), self);
    }
}

/// Result from process selection
enum SelectionResult {
    Quit,
    AttachTo(Pid),
}

struct TaskHandles {
    term_handle: tokio::task::JoinHandle<()>,
    periodic_handle: tokio::task::JoinHandle<()>,
    sampler_handle: tokio::task::JoinHandle<()>,
}

impl TaskHandles {
    fn abort(self) {
        self.term_handle.abort();
        self.periodic_handle.abort();
        self.sampler_handle.abort();
    }
}

impl ProfilingState {
    pub(crate) async fn run_attached<S: SamplerOps>(
        &mut self,
        terminal: &mut DefaultTerminal,
        sampler: S,
    ) -> Result<(), Error> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<UpdateEvent>(2);
        let handles = self.run_event_senders(event_tx, sampler);

        let result = self.run_until_error(terminal, &mut event_rx).await;

        // Abort spawned tasks before returning to avoid lingering EventStream
        handles.abort();

        result
    }

    fn run_event_senders<S: SamplerOps>(
        &self,
        sender: tokio::sync::mpsc::Sender<UpdateEvent>,
        sampler: S,
    ) -> TaskHandles {
        // Terminal event sender
        let term_handle = tokio::spawn({
            let tx = sender.clone();
            async move {
                let _ = send_terminal_event(tx).await;
            }
        });

        // Sampler task
        let queue = Arc::clone(&self.record_queue_map);
        let sampler_handle = tokio::spawn(async move {
            let _ = sampler.push_to_queue(queue).await;
        });

        // Periodic refresh
        let update_period = self.update_period;
        let periodic_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_period);
            loop {
                interval.tick().await;
                if sender.send(UpdateEvent::Periodic).await.is_err() {
                    break;
                }
            }
        });

        TaskHandles {
            term_handle,
            periodic_handle,
            sampler_handle,
        }
    }
}
