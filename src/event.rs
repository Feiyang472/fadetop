use anyhow::Error;
use ratatui::crossterm;

use crate::{
    app::{FadeTopApp, SamplerFactory},
    errors::AppError,
};

use crossterm::event::{Event, KeyCode, KeyEventKind};
pub enum UpdateEvent {
    Periodic,
    Input(crossterm::event::Event),
    Error(AppError),
}

impl UpdateEvent {
    pub fn update_state<F>(self, app: &mut FadeTopApp<F>) -> Result<(), Error>
    where
        F: SamplerFactory,
    {
        match self {
            UpdateEvent::Input(term_event) => UpdateEvent::handle_crossterm_events(term_event, app),
            UpdateEvent::Periodic => Ok(()),
            UpdateEvent::Error(err) => Err(err.into()),
        }
    }

    fn handle_crossterm_events<F>(term_event: Event, app: &mut FadeTopApp<F>) -> Result<(), Error>
    where
        F: SamplerFactory,
    {
        match term_event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Right => app.app_state.next_tab(),
                KeyCode::Left => app.app_state.prev_tab(),
                KeyCode::Esc => Ok(app.app_state.quit()),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}
