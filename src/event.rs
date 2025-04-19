use anyhow::Error;
use ratatui::crossterm::{self};

use crate::{
    app::FadeTopApp,
    errors::AppError,
};

use crossterm::event::{Event, KeyCode};
pub enum UpdateEvent {
    Periodic,
    Input(crossterm::event::Event),
    Error(AppError),
}

impl UpdateEvent {
    pub fn update_state(self, app: &mut FadeTopApp) -> Result<(), Error> {
        match self {
            UpdateEvent::Input(term_event) => UpdateEvent::handle_crossterm_events(term_event, app),
            UpdateEvent::Periodic => Ok(()),
            UpdateEvent::Error(err) => Err(err.into()),
        }
    }

    fn handle_crossterm_events(term_event: Event, app: &mut FadeTopApp) -> Result<(), Error> {
        match term_event {
            Event::Key(key) => match key.code {
                KeyCode::Right => app.app_state.next_tab(),
                KeyCode::Left => app.app_state.prev_tab(),
                KeyCode::Up => Ok(app.app_state.viewport_bound.zoom_out()),
                KeyCode::Down => Ok(app.app_state.viewport_bound.zoom_in()),
                KeyCode::Esc => Ok(app.app_state.quit()),
                KeyCode::Char('a') => Ok(app.app_state.viewport_bound.move_left()),
                KeyCode::Char('d') => Ok(app.app_state.viewport_bound.move_right()),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}
