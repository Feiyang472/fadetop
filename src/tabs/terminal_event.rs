use anyhow::Error;
use ratatui::{crossterm, crossterm::event};

use crate::{
    app::FadeTopApp,
    errors::AppError,
    state::{AppState, Focus},
};

pub enum UpdateEvent {
    Periodic,
    Input(crossterm::event::Event),
    Error(AppError),
}

impl AppState {
    fn handle_crossterm_events(&mut self, term_event: event::Event) -> Result<(), Error> {
        match term_event {
            event::Event::Key(key) => match key.code {
                // Global shortcuts
                event::KeyCode::Esc => Ok(self.quit()),
                event::KeyCode::Tab => {
                    self.focus = match self.focus {
                        Focus::ThreadList => Focus::Timeline,
                        Focus::Timeline => Focus::LogView,
                        Focus::LogView => Focus::ThreadList,
                    };
                    Ok(())
                }
                _ => Ok({
                    match self.focus {
                        Focus::ThreadList => self.thread_selection.handle_key_event(&key),
                        Focus::Timeline => self.viewport_bound.handle_key_event(&key),
                        _ => {}
                    }
                }),
            },
            _ => Ok(()),
        }
    }

    fn handle_periodic_tick(&mut self) -> Result<(), Error> {
        let num_threads = self
            .record_queue_map
            .read()
            .map_err(|_| std::sync::PoisonError::new(()))?
            .len();
        self.thread_selection.num_threads = num_threads;
        Ok(())
    }
}

impl UpdateEvent {
    pub fn update_state(self, app: &mut FadeTopApp) -> Result<(), Error> {
        match self {
            UpdateEvent::Input(term_event) => app.app_state.handle_crossterm_events(term_event),
            UpdateEvent::Periodic => app.app_state.handle_periodic_tick(),
            UpdateEvent::Error(err) => Err(err.into()),
        }
    }
}
