use anyhow::Error;
use ratatui::crossterm;

use crate::app::{FadeTopApp, SamplerFactory};

use crossterm::event::{Event, KeyCode, KeyEventKind};
pub enum UpdateEvent {
    Periodic,
    Input(crossterm::event::Event),
}

impl UpdateEvent {
    pub fn update_state<F>(self, app: &mut FadeTopApp<F>) -> Result<(), Error>
    where
        F: SamplerFactory,
    {
        match self {
            UpdateEvent::Input(term_event) => UpdateEvent::handle_crossterm_events(term_event, app),
            UpdateEvent::Periodic => Ok(()),
        }
    }

    fn handle_crossterm_events<F>(term_event: Event, app: &mut FadeTopApp<F>) -> Result<(), Error>
    where
        F: SamplerFactory,
    {
        match term_event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Right => app.tab_selection_state.next_tab(),
                KeyCode::Left => app.tab_selection_state.prev_tab(),
                KeyCode::Esc => app.quit(),
                _ => {}
            },
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }
}
