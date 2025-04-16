use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Error;
use ratatui::prelude::Stylize;
use ratatui::{
    text::Line,
    widgets::{Block, Borders, ScrollbarState},
};

use crate::priority::{ForgettingQueue, ForgettingQueueMap};

#[derive(Debug, Clone, Copy)]
pub enum ViewPortRight {
    Latest,
    Selected(Instant),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewPortBounds {
    pub right: ViewPortRight,
    pub width: Duration,
}

impl ViewPortBounds {
    pub fn render_header(&self, queue: &ForgettingQueue) -> Block {
        Block::default()
            .title(
                Line::from(format!(
                    "<-{:0>2}:{:0>2}->",
                    (self.width).as_secs() / 60,
                    (self.width).as_secs()
                ))
                .bold()
                .centered(),
            )
            .title(
                Line::from(match self.right {
                    ViewPortRight::Latest => "Now".to_string(),
                    ViewPortRight::Selected(right) => {
                        let window_right = (queue.last_update - right).as_secs();
                        format!("-{:0>2}:{:0>2}", window_right / 60, window_right)
                    }
                })
                .right_aligned(),
            )
            .title(
                Line::from({
                    let furthest_left = (queue.last_update - queue.start_ts).as_secs();
                    format!("-{:0>2}:{:0>2}", furthest_left / 60, furthest_left,)
                })
                .left_aligned(),
            )
            .borders(Borders::TOP)
    }
}

#[derive(Debug)]
pub struct AppState {
    pub selected_tab: usize,
    pub forgetting_queues: Arc<RwLock<ForgettingQueueMap>>,
    pub stack_level_scroll_state: ScrollbarState,
    pub(crate) viewport_bound: ViewPortBounds,
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
            viewport_bound: ViewPortBounds {
                right: ViewPortRight::Latest,
                width: Duration::from_secs(10),
            },
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
