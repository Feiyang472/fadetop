use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, StatefulWidget, Widget},
};

pub mod local_variables;
pub mod terminal_event;
pub mod thread_selection;
pub mod timeline;

pub struct Blocked<W> {
    sub: W,
}

impl<W: Widget> Widget for Blocked<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default();
        let inner = block.inner(area);
        block.render(area, buf);
        self.sub.render(inner, buf);
    }
}

pub trait StatefulWidgetExt: StatefulWidget + Sized {
    fn get_block(&self, _state: &mut Self::State) -> Block {
        Default::default()
    }

    fn blocked<'b>(self) -> Blocked<Self> {
        Blocked { sub: self }
    }
}

impl<W: StatefulWidgetExt> StatefulWidget for Blocked<W> {
    type State = W::State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = self.sub.get_block(state);

        let inner = block.inner(area);
        block.render(area, buf);
        self.sub.render(inner, buf, state);
    }
}

pub(super) fn get_scroll(x: u16, capacity: u16) -> u16 {
    x.saturating_div(capacity.max(1)) * capacity.max(1)
}
