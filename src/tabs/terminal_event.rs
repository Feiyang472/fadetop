use anyhow::Error;
use ratatui::{crossterm, crossterm::event};
use remoteprocess::Tid;

use crate::{
    app::FadeTopApp,
    errors::AppError,
    priority::SpiedRecordQueue,
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
                        Focus::LogView => self.local_variable_state.handle_key_event(&key),
                    }
                }),
            },
            _ => Ok(()),
        }
    }

    fn handle_periodic_tick(&mut self) -> Result<(), Error> {
        let qmaps = self
            .record_queue_map
            .read()
            .map_err(|_| std::sync::PoisonError::new(()))?;

        self.thread_selection
            .available_threads
            .retain(|tinfo| qmaps.contains_key(&tinfo.tid));
        let mut sorted_qmaps: Vec<(&Tid, &SpiedRecordQueue)> = qmaps.iter().collect();
        sorted_qmaps.sort_by(|(_, q1), (_, q2)| q1.thread_info.pid.cmp(&q2.thread_info.pid));
        for (tid, q) in sorted_qmaps {
            if let None = self
                .thread_selection
                .available_threads
                .iter()
                .find(|tinfo| tinfo.tid == *tid)
            {
                self.thread_selection
                    .available_threads
                    .push(q.thread_info.clone());
            }
        }
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
