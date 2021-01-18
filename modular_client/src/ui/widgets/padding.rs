use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, context::Context, edge_insets::EdgeInsets, rect::Rect, size::Size, widget::Widget};

#[derive(Debug)]
pub struct Padding {
    pub child: Box<dyn Widget>,
    pub padding: EdgeInsets,
    size: Size,
}

impl Padding {
    pub fn new(padding: EdgeInsets, child: impl Widget + 'static) -> Self {
        Padding {
            child: Box::new(child),
            padding,
            size: Size::zero(),
        }
    }
}

impl Widget for Padding {
    fn layout(&mut self, constraints: BoxConstraints, canvas: &mut Canvas<OpenGl>, context: &Context) -> Size {
        self.size = constraints.biggest();
        let deflated_constraints = constraints.deflate(self.padding);
        self.child.layout(deflated_constraints, canvas, context);
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        canvas.save_with(|canvas| {
            let inner_rect = self
                .padding
                .deflate_rect(Rect::from_lt_size(0.0, 0.0, self.size));
            canvas.translate(inner_rect.left, inner_rect.top);
            self.child.paint(canvas, context);
        })
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
