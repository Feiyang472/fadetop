use anyhow::Error;
use ratatui::crossterm;

use crate::{errors::AppError, state::AppState};

pub enum UpdateEvent {
    Periodic,
    Input(crossterm::event::Event),
    Error(AppError),
}

impl UpdateEvent {
    pub fn update_state(self, app_state: &mut AppState) -> Result<(), Error> {
        match self {
            UpdateEvent::Input(term_event) => app_state.handle_crossterm_events(term_event),
            UpdateEvent::Periodic => Ok(()),
            UpdateEvent::Error(err) => Err(err.into()),
        }
    }
}
