use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct Clip {
    pub child: Box<dyn Widget>,
    size: Size,
}

impl Clip {
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            size: Size::zero(),
        }
    }
}

impl Widget for Clip {
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
            canvas.scissor(0.0, 0.0, self.size.width, self.size.height);
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
