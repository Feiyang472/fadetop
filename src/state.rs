use std::sync::{Arc, RwLock};

use crate::{
    priority::SpiedRecordQueueMap,
    tabs::{
        local_variables::LocalVariableSelection, thread_selection::ThreadSelectionState,
        timeline::ViewPortBounds,
    },
};

// Add a Focus enum to track current focus
#[derive(Debug, PartialEq, Eq)]
pub enum Focus {
    ThreadList,
    Timeline,
    LogView,
}

#[derive(Debug)]
pub struct AppState {
    pub(crate) focus: Focus,
    pub(crate) thread_selection: ThreadSelectionState,
    pub(crate) viewport_bound: ViewPortBounds,
    pub(crate) local_variable_state: LocalVariableSelection,
    pub record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
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
            focus: Focus::ThreadList,
            thread_selection: Default::default(),
            record_queue_map: Default::default(),
            viewport_bound: Default::default(),
            local_variable_state: LocalVariableSelection::default(),
            running: true,
        }
    }
}
