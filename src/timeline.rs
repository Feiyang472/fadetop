use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::{Color, Style},
    widgets::{
        Block, Borders, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

use crate::state::AppState;

pub struct TimelineWidget {}

impl StatefulWidget for TimelineWidget {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default().title("Timeline").borders(Borders::TOP);
        block.clone().render(area, buf);

        if let Some((_tid, queue)) = state
            .forgetting_queues
            .write()
            .unwrap()
            .iter()
            .nth(state.selected_tab)
        {
            // Render finished events

            let total_duration = queue.last_update - queue.start_ts;

            let (visible_end, window_width) = (
                state.viewport_time_bound.0.unwrap_or(queue.last_update),
                state.viewport_time_bound.1.min(total_duration),
            );

            let visible_start = visible_end - window_width;

            state.time_scroll_state = state
                .time_scroll_state
                // for some reason content length has to be the length of the content not in the current viewport
                .content_length((total_duration - window_width).as_micros() as usize)
                .position((visible_start - queue.start_ts).as_micros() as usize)
                .viewport_content_length(window_width.as_micros() as usize);

            self.render_scrollbar(area, buf, &mut state.time_scroll_state);

            let inner = block.inner(area);
            let width = inner.width as usize;
            queue.finished_events.iter().for_each(|record| {
                if record.start > visible_end {
                    return;
                }
                if record.end < visible_start {
                    return;
                }
                render_event(
                    buf,
                    inner,
                    (record.start - visible_start).as_micros() as usize,
                    (record.end - visible_start).as_micros() as usize,
                    record.depth as u16,
                    &record.frame_key.name,
                    window_width.as_micros() as usize,
                    width,
                    Color::Blue,
                );
            });

            for (depth, record) in queue.unfinished_events.iter().enumerate() {
                render_event(
                    buf,
                    inner,
                    (record.start - visible_start).as_micros() as usize,
                    window_width.as_micros() as usize, // Use window_width.as_micros() as usize as the "end" for unfinished events
                    depth as u16,
                    &record.frame_key.name,
                    window_width.as_micros() as usize,
                    width,
                    Color::Red,
                );
            }
        }
    }
}

impl TimelineWidget {
    pub fn render_scrollbar(&self, area: Rect, buf: &mut Buffer, state: &mut ScrollbarState) {
        Scrollbar::default()
            .orientation(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("a"))
            .end_symbol(Some("d"))
            .render(
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                buf,
                state,
            );
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
    if total_duration == 0 {
        return;
    }
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
