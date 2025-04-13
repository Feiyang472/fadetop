use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use ratatui::widgets::ScrollbarState;

use crate::priority::ForgettingQueueMap;

#[derive(Debug)]
pub struct AppState {
    pub selected_tab: usize,
    pub forgetting_queues: Arc<RwLock<ForgettingQueueMap>>,
    pub stack_level_scroll_state: ScrollbarState,
    pub time_scroll_state: ScrollbarState,
    pub(crate) viewport_time_bound: (Option<Instant>, Duration),
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            forgetting_queues: Arc::new(RwLock::new(HashMap::default())),
            stack_level_scroll_state: ScrollbarState::default(),
            time_scroll_state: ScrollbarState::default(),
            viewport_time_bound: (None, Duration::from_secs(10)),
        }
    }

    fn num_threads(&self) -> usize {
        self.forgetting_queues.read().unwrap().len()
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self
            .selected_tab
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads())
            .unwrap_or(0)
    }
    pub fn prev_tab(&mut self) {
        self.selected_tab = self
            .selected_tab
            .overflowing_add(self.num_threads().saturating_sub(1))
            .0
            .checked_rem(self.num_threads())
            .unwrap_or(0)
    }
}
