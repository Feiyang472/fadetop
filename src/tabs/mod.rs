use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, StatefulWidget, Widget},
};

pub mod local_variables;
pub mod terminal_event;
pub mod thread_selection;
pub mod timeline;

pub struct Blocked<'b, W> {
    sub: W,
    block: Block<'b>,
}

impl<W: Widget> Widget for Blocked<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = self.block.inner(area);
        self.block.render(area, buf);
        self.sub.render(inner, buf);
    }
}
impl<W: StatefulWidget> StatefulWidget for Blocked<'_, W> {
    type State = W::State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.block.inner(area);
        self.block.render(area, buf);
        self.sub.render(inner, buf, state);
    }
}
