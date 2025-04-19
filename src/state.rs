use std::{
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
    pub selected_depth: u32,
}

impl Default for ViewPortBounds {
    fn default() -> Self {
        Self {
            right: ViewPortRight::Latest,
            width: Duration::from_secs(60),
            selected_depth: 0,
        }
    }
}

impl ViewPortBounds {
    pub fn zoom_in(&mut self) {
        self.width = self.width.mul_f32(1.5);
    }

    pub fn zoom_out(&mut self) {
        self.width = self.width.div_f32(1.5);
    }

    pub fn move_left(&mut self) {
        match self.right {
            ViewPortRight::Latest => {
                self.right = ViewPortRight::Selected(Instant::now() - self.width / 2);
            }
            ViewPortRight::Selected(right) => {
                self.right = ViewPortRight::Selected(right - self.width / 2);
            }
        }
    }

    pub fn move_right(&mut self) {
        match self.right {
            ViewPortRight::Latest => {
                self.right = ViewPortRight::Selected(Instant::now() + self.width / 2);
            }
            ViewPortRight::Selected(right) => {
                self.right = ViewPortRight::Selected(right + self.width / 2);
            }
        }
    }

    pub fn get_block(&self, queue: &ForgettingQueue) -> Block {
        Block::default()
            .title(
                Line::from(format!(
                    "<-{:0>2}:{:0>2}->",
                    (self.width).as_secs() / 60,
                    (self.width).as_secs() % 60
                ))
                .bold()
                .centered(),
            )
            .title(
                Line::from(match self.right {
                    ViewPortRight::Latest => "Now".to_string(),
                    ViewPortRight::Selected(right) => {
                        let window_right = (queue.last_update - right).as_secs();
                        format!("-{:0>2}:{:0>2}", window_right / 60, window_right % 60)
                    }
                })
                .right_aligned(),
            )
            .title(
                Line::from({
                    let furthest_left = (queue.last_update - queue.start_ts).as_secs();
                    format!("-{:0>2}:{:0>2}", furthest_left / 60, furthest_left % 60)
                })
                .left_aligned(),
            )
            .borders(Borders::ALL)
    }
}

#[derive(Debug)]
pub struct AppState {
    pub selected_thread: usize,
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
            selected_thread: 0,
            forgetting_queues: Arc::default(),
            stack_level_scroll_state: ScrollbarState::default(),
            viewport_bound: ViewPortBounds::default(),
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
        self.selected_thread = self
            .selected_thread
            .overflowing_add(1)
            .0
            .checked_rem(self.num_threads()?)
            .unwrap_or(0);
        Ok(())
    }
    pub fn prev_tab(&mut self) -> Result<(), Error> {
        let num_threads = self.num_threads()?;
        self.selected_thread = self
            .selected_thread
            .overflowing_add(num_threads.saturating_sub(1))
            .0
            .checked_rem(num_threads)
            .unwrap_or(0);
        Ok(())
    }
}
