use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, StatefulWidget, Widget},
};

use crate::state::AppState;

pub struct TimelineWidget {}

impl StatefulWidget for TimelineWidget {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .title("Timeline")
            .borders(ratatui::widgets::Borders::ALL);
        block.clone().render(area, buf);

        let inner = block.inner(area);

        let queue = state.forgetting_queue.write().unwrap();

        let total_duration = (Instant::now() - queue.start_ts).as_micros() as usize;
        let width = inner.width as usize;

        if let Some((tid, records)) = queue.unfinished_events.iter().nth(state.selected_tab) {
            // Render finished events
            queue.finished_events.iter().for_each(|record| {
                if tid != &record.frame_key.tid {
                    return;
                }
                render_event(
                    buf,
                    inner,
                    (record.start - queue.start_ts).as_micros() as usize,
                    (record.end - queue.start_ts).as_micros() as usize,
                    record.depth as u16,
                    &record.frame_key.name,
                    total_duration,
                    width,
                    Color::Blue,
                );
            });

            for (depth, record) in records.iter().enumerate() {
                render_event(
                    buf,
                    inner,
                    (record.start - queue.start_ts).as_micros() as usize,
                    total_duration, // Use total_duration as the "end" for unfinished events
                    depth as u16,
                    &record.frame_key.name,
                    total_duration,
                    width,
                    Color::Red,
                );
            }
        }
    }
}

// Reusable function to render an event
fn render_event(
    buf: &mut Buffer,
    inner: Rect,
    start: usize,
    end: usize,
    depth: u16,
    name: &str,
    total_duration: usize,
    width: usize,
    color: Color,
) {
    // Calculate relative positions
    let relative_start = (start * width) / total_duration;
    let relative_end = (end * width) / total_duration;

    // Ensure the range is within bounds
    let x_start = inner.left() + relative_start as u16;
    let x_end = inner.left() + relative_end as u16;

    // Render the block as a padded string
    if x_end > x_start {
        let block_width = (x_end - x_start) as usize;

        // Create a string padded with spaces to fit the width
        let padded_string = format!(
            "{:^block_width$}",
            name.chars().take(block_width).collect::<String>(),
            block_width = block_width
        );

        // Render the padded string
        buf.set_string(
            x_start,
            inner.top() + depth,
            padded_string,
            Style::default().fg(Color::White).bg(color),
        );
    }
}
