use std::sync::{Arc, RwLock};

use crate::priority::ForgettingQueue;

#[derive(Debug)]
pub struct AppState {
    pub selected_tab: usize,
    pub forgetting_queue: Arc<RwLock<ForgettingQueue>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            forgetting_queue: Arc::new(RwLock::new(ForgettingQueue::new())),
        }
    }

    fn num_threads(&self) -> usize {
        self.forgetting_queue
            .read()
            .unwrap()
            .unfinished_events
            .len()
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
