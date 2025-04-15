use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Error;
use ratatui::widgets::ScrollbarState;

use crate::priority::ForgettingQueueMap;

#[derive(Debug)]
pub struct AppState {
    pub selected_tab: usize,
    pub forgetting_queues: Arc<RwLock<ForgettingQueueMap>>,
    pub stack_level_scroll_state: ScrollbarState,
    pub(crate) viewport_time_bound: (Option<Instant>, Duration),
    running: bool,
}

impl AppState {
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            forgetting_queues: Arc::new(RwLock::new(HashMap::default())),
            stack_level_scroll_state: ScrollbarState::default(),
            viewport_time_bound: (None, Duration::from_secs(10)),
            running: true,
        }
    }

    fn num_threads(&self) -> Result<usize, Error> {
        Ok(self
            .forgetting_queues
            .read()
            .map_err(|_| std::sync::PoisonError::new(()))?
            .len())
    }

    pub fn next_tab(&mut self) -> Result<(), Error> {
        self.selected_tab = self
            .selected_tab
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads()?)
            .unwrap_or(0);
        Ok(())
    }
    pub fn prev_tab(&mut self) -> Result<(), Error> {
        let num_threads = self.num_threads()?;
        self.selected_tab = self
            .selected_tab
            .overflowing_add(num_threads.saturating_sub(1))
            .0
            .checked_rem(num_threads)
            .unwrap_or(0);
        Ok(())
    }
}
