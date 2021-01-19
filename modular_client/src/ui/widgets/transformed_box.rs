use femtovg::{renderer::OpenGl, Canvas, Transform2D};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct TransformedBox {
    pub transform: Transform2D,
    pub child: Box<dyn Widget>,
    size: Size,
}

impl TransformedBox {
    pub fn new(transform: Transform2D, child: impl Widget + 'static) -> Self {
        Self {
            transform,
            child: Box::new(child),
            size: Size::zero(),
        }
    }
}

impl Widget for TransformedBox {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        self.size = self.child.layout(constraints, canvas, context);
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        canvas.save_with(|canvas| {
            let [a, b, c, d, e, f] = self.transform.0;
            canvas.set_transform(a, b, c, d, e, f);
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
