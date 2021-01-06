use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{
    box_constraints::BoxConstraints, edge_insets::EdgeInsets, rect::Rect, size::Size,
    widget::Widget,
};

pub struct Padding {
    pub child: Box<dyn Widget>,
    pub padding: EdgeInsets,
    size: Size,
}

impl Padding {
    pub fn new(child: Box<dyn Widget>, padding: EdgeInsets) -> Box<Self> {
        Box::new(Padding {
            child,
            padding,
            size: Size::zero(),
        })
    }
}

impl Widget for Padding {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        self.size = constraints.biggest();
        let deflated_constraints = constraints.deflate(self.padding);
        self.child.layout(&deflated_constraints, canvas);
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        canvas.save_with(|canvas| {
            let inner_rect = self
                .padding
                .deflate_rect(Rect::from_lt_size(0.0, 0.0, self.size));
            canvas.translate(inner_rect.left, inner_rect.top);
            canvas.scissor(0.0, 0.0, inner_rect.width(), inner_rect.height());
            self.child.paint(canvas);
        })
    }

    fn size(&self) -> Size {
        self.size
    }
}
