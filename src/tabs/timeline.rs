use std::{
    ops::{DivAssign, MulAssign},
    time::{Duration, Instant},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyEvent},
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::{
    priority::SpiedRecordQueue,
    state::{AppState, Focus},
};

#[derive(Debug, Clone, Copy)]
enum ViewPortRight {
    Latest,
    Selected(Instant),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewPortBounds {
    right: ViewPortRight,
    pub(crate) width: Duration,
    pub(super) selected_depth: u16,
}

impl Default for ViewPortBounds {
    fn default() -> Self {
        Self {
            right: ViewPortRight::Latest,
            width: Duration::from_secs(60),
            selected_depth: 0,
        }
    }
}

impl ViewPortBounds {
    fn zoom_out(&mut self) {
        self.width.mul_assign(2);
    }

    fn zoom_in(&mut self) {
        self.width.div_assign(2);
    }

    fn move_left(&mut self) {
        match self.right {
            ViewPortRight::Latest => {
                self.right = ViewPortRight::Selected(Instant::now() - self.width / 2);
            }
            ViewPortRight::Selected(right) => {
                self.right = ViewPortRight::Selected(right - self.width / 2);
            }
        }
    }

    fn move_right(&mut self) {
        match self.right {
            ViewPortRight::Latest => {
                self.right = ViewPortRight::Selected(Instant::now() + self.width / 2);
            }
            ViewPortRight::Selected(right) => {
                self.right = ViewPortRight::Selected(right + self.width / 2);
            }
        }
    }

    fn move_up(&mut self) {
        if self.selected_depth > 0 {
            self.selected_depth -= 1;
        }
    }

    fn move_down(&mut self) {
        self.selected_depth += 1;
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) {
        match key.code {
            event::KeyCode::Char('o') => self.zoom_out(),
            event::KeyCode::Char('i') => self.zoom_in(),
            event::KeyCode::Left => self.move_left(),
            event::KeyCode::Right => self.move_right(),
            event::KeyCode::Up => self.move_up(),
            event::KeyCode::Down => self.move_down(),
            _ => {}
        }
    }

    fn get_block(&self, queue: &SpiedRecordQueue, focused: bool) -> Block {
        Block::default()
            .title(
                Line::from(format!(
                    "<-{:0>2}:{:0>2}->",
                    (self.width).as_secs() / 60,
                    (self.width).as_secs() % 60
                ))
                .bold()
                .centered(),
            )
            .title(
                Line::from(match self.right {
                    ViewPortRight::Latest => "Now".to_string(),
                    ViewPortRight::Selected(right) => {
                        let window_right = (queue.last_update - right).as_secs();
                        format!("-{:0>2}:{:0>2}", window_right / 60, window_right % 60)
                    }
                })
                .right_aligned(),
            )
            .title(
                Line::from({
                    let furthest_left = (queue.last_update - queue.start_ts).as_secs();
                    format!("-{:0>2}:{:0>2}", furthest_left / 60, furthest_left % 60)
                })
                .left_aligned(),
            )
            .borders(Borders::TOP | Borders::RIGHT)
            .border_style(if focused {
                Style::new().blue().on_white().bold().italic()
            } else {
                Style::default()
            })
    }
}

pub struct TimelineWidget {}

impl StatefulWidget for TimelineWidget {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Ok(queues) = state.record_queue_map.read() {
            if let Some(queue) = state.thread_selection.select_thread(&queues) {
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

                let block = state
                    .viewport_bound
                    .get_block(&queue, state.focus == Focus::Timeline);

                let inner = block.inner(area);
                block.render(area, buf);

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
                            get_color_for_name(&record.frame_key.fqn()),
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
                            Color::Rgb(
                                0,
                                150 - ((depth % 8 * 16) as u8),
                                200 - ((depth % 8 * 16) as u8),
                            ),
                        );
                    });

                buf.cell_mut((
                    inner.right(),
                    inner.top() + state.viewport_bound.selected_depth,
                ))
                .map(|cell| cell.set_bg(Color::DarkGray).set_char('→'));
            }
        } else {
            state.quit();
        }
    }
}

fn get_color_for_name(name: &str) -> Color {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let hue = (hash % 360) as f32;
    let saturation = 0.35;
    let lightness = 0.6;

    let c = (1.0_f32 - (2.0_f32 * lightness - 1.0_f32).abs()) * saturation;
    let h = hue / 60.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = lightness - c / 2.0;

    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::Rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
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
    color: Color,
) {
    if window_width == 0 {
        return;
    }

    let tab_width = inner.width as usize;

    let relative_start = (start * tab_width) / window_width;
    let relative_end = ((end * tab_width).div_ceil(window_width)).min(tab_width);

    let x_start = inner.left() + relative_start as u16;
    let x_end = inner.left() + relative_end as u16;

    if x_end > x_start + 2 {
        // block width is unstable due to rounding
        let block_width = relative_end - relative_start;

        let padded_string = format!(
            "{:^block_width$}",
            name.chars().take(block_width).collect::<String>(),
            block_width = block_width
        );

        buf.set_string(
            x_start,
            inner.top() + depth,
            padded_string,
            Style::default().fg(Color::White).bg(color),
        );
    }
}
