use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{StatefulWidget, Widget},
};

use crate::state::{AppState, ViewPortRight};

pub struct TimelineWidget {}

impl StatefulWidget for TimelineWidget {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Ok(queues) = state.forgetting_queues.read() {
            if let Some((_tid, queue)) = queues.iter().nth(state.selected_tab) {
                if let ViewPortRight::Selected(end) = state.viewport_bound.right {
                    if end > queue.last_update {
                        state.viewport_bound.right = ViewPortRight::Latest;
                    }
                }
                let visible_end = match state.viewport_bound.right {
                    ViewPortRight::Selected(end) => end,
                    ViewPortRight::Latest => queue.last_update,
                };

                let window_width = state.viewport_bound.width;
                let visible_start = visible_end - window_width;

                let block = state.viewport_bound.render_header(&queue);

                let inner = block.inner(area);
                block.render(area, buf);

                let width = inner.width as usize;

                queue.finished_events.iter().for_each(|record| {
                    if record.start <= visible_end
                        && record.end >= visible_start
                        && record.depth < inner.height as usize
                    {
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
                    }
                });

                queue
                    .unfinished_events
                    .iter()
                    .take(inner.height as usize)
                    .enumerate()
                    .for_each(|(depth, record)| {
                        render_event(
                            buf,
                            inner,
                            (record.start - visible_start).as_micros() as usize,
                            window_width.as_micros() as usize,
                            depth as u16,
                            &record.frame_key.name,
                            window_width.as_micros() as usize,
                            width,
                            Color::Red,
                        );
                    });
            }
        } else {
            state.quit();
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
    window_width: usize,
    width: usize,
    color: Color,
) {
    if window_width == 0 {
        return;
    }
    // Calculate relative positions
    let relative_start = (start * width) / window_width;
    let relative_end = (end * width) / window_width;

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
