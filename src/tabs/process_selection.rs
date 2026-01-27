use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};
use remoteprocess::Pid;

use crate::processes::{ProcessDiscovery, PythonProcess};

use super::{StatefulWidgetExt, get_scroll};

/// Actions that can result from process selection events
#[derive(Debug, Clone)]
pub enum ProcessSelectionAction {
    None,
    AttachTo(Pid),
    Refresh,
    Quit,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessSelectionState {
    processes: Vec<PythonProcess>,
    selected_index: usize,
    filter_text: String,
    filter_mode: bool,
    /// Error message to display (e.g., failed to attach)
    pub error_message: Option<String>,
}

impl ProcessSelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Refresh the process list using the given discovery implementation
    pub fn refresh_processes<D: ProcessDiscovery>(&mut self) {
        self.processes = D::discover();
        // Clamp selected index if list shrunk
        let filtered_len = self.filtered_processes().len();
        if self.selected_index >= filtered_len && filtered_len > 0 {
            self.selected_index = filtered_len - 1;
        }
    }

    /// Returns filtered processes based on current filter
    pub fn filtered_processes(&self) -> Vec<&PythonProcess> {
        if self.filter_text.is_empty() {
            self.processes.iter().collect()
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            self.processes
                .iter()
                .filter(|p| p.cmdline.to_lowercase().contains(&filter_lower))
                .collect()
        }
    }

    /// Get the currently selected process PID
    pub fn selected_pid(&self) -> Option<Pid> {
        self.filtered_processes()
            .get(self.selected_index)
            .map(|p| p.pid)
    }

    /// Handle keyboard events when this widget has focus
    pub fn handle_focused_event(&mut self, key: &KeyEvent) -> ProcessSelectionAction {
        if self.filter_mode {
            self.handle_filter_mode_event(key)
        } else {
            self.handle_normal_mode_event(key)
        }
    }

    fn handle_filter_mode_event(&mut self, key: &KeyEvent) -> ProcessSelectionAction {
        match key.code {
            event::KeyCode::Esc => {
                self.filter_mode = false;
                ProcessSelectionAction::None
            }
            event::KeyCode::Enter => {
                self.filter_mode = false;
                ProcessSelectionAction::None
            }
            event::KeyCode::Backspace => {
                self.filter_text.pop();
                self.selected_index = 0;
                ProcessSelectionAction::None
            }
            event::KeyCode::Char(c) => {
                self.filter_text.push(c);
                self.selected_index = 0;
                ProcessSelectionAction::None
            }
            _ => ProcessSelectionAction::None,
        }
    }

    fn handle_normal_mode_event(&mut self, key: &KeyEvent) -> ProcessSelectionAction {
        // Clear error message on any key press
        self.error_message = None;

        match key.code {
            event::KeyCode::Down | event::KeyCode::Char('j') => {
                let max = self.filtered_processes().len().saturating_sub(1);
                self.selected_index = (self.selected_index + 1).min(max);
                ProcessSelectionAction::None
            }
            event::KeyCode::Up | event::KeyCode::Char('k') => {
                self.selected_index = self.selected_index.saturating_sub(1);
                ProcessSelectionAction::None
            }
            event::KeyCode::Enter => {
                if let Some(pid) = self.selected_pid() {
                    ProcessSelectionAction::AttachTo(pid)
                } else {
                    ProcessSelectionAction::None
                }
            }
            event::KeyCode::Char('/') => {
                self.filter_mode = true;
                ProcessSelectionAction::None
            }
            event::KeyCode::Char('r') => ProcessSelectionAction::Refresh,
            event::KeyCode::Esc => ProcessSelectionAction::Quit,
            _ => ProcessSelectionAction::None,
        }
    }

    /// Set an error message to display
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }
}

pub struct ProcessSelectionWidget;

impl StatefulWidget for ProcessSelectionWidget {
    type State = ProcessSelectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }

        let filtered = state.filtered_processes();

        if filtered.is_empty() {
            let msg = if state.processes.is_empty() {
                "No Python processes found. Press 'r' to refresh."
            } else {
                "No matches. Press Backspace or Esc to clear filter."
            };
            Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let lines: Vec<Line> = filtered
            .iter()
            .enumerate()
            .map(|(i, proc)| {
                let pid_span = Span::styled(
                    format!("{:>8} ", proc.pid),
                    Style::default().fg(Color::Cyan),
                );
                let cmd_span = Span::raw(&proc.cmdline);

                let mut line = Line::from(vec![pid_span, cmd_span]);

                if i == state.selected_index {
                    line = line.bg(Color::Blue).bold();
                }

                line
            })
            .collect();

        let scroll_offset = get_scroll(state.selected_index as u16, area.height);

        Paragraph::new(lines)
            .scroll((scroll_offset, 0))
            .render(area, buf);
    }
}

impl StatefulWidgetExt for ProcessSelectionWidget {
    fn get_block(&self, state: &mut Self::State) -> Block<'_> {
        let title = format!("Select Python Process ({} found)", state.processes.len());

        let mut block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().blue());

        // Show error message if present
        if let Some(ref error) = state.error_message {
            block = block.title_bottom(
                Line::from(error.clone())
                    .centered()
                    .style(Style::default().fg(Color::Red).bold()),
            );
            return block;
        }

        // Show filter or help text at bottom
        if state.filter_mode {
            let filter_line = format!("Filter: {}_", state.filter_text);
            block = block.title_bottom(
                Line::from(filter_line)
                    .left_aligned()
                    .style(Style::default().fg(Color::Yellow)),
            );
        } else if !state.filter_text.is_empty() {
            let filter_display = format!("Filter: {} (/ to edit, Esc to clear)", state.filter_text);
            block = block.title_bottom(Line::from(filter_display).left_aligned());
        } else {
            block = block
                .title_bottom(Line::from("Enter:attach  /:filter  r:refresh  Esc:quit").centered());
        }

        block
    }
}
