use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{
    box_constraints::BoxConstraints, context::Context, paint::PaintFnTr, size::Size, widget::Widget,
};

#[derive(Debug)]
pub struct CustomPaint {
    pub paint_fn: Box<dyn PaintFnTr<Output = ()>>,
    size: Size,
}

impl CustomPaint {
    pub fn new(paint_fn: impl PaintFnTr<Output = ()> + 'static) -> Self {
        Self {
            paint_fn: Box::new(paint_fn),
            size: Size::zero(),
        }
    }
}

impl Widget for CustomPaint {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        _canvas: &mut Canvas<OpenGl>,
        _context: &Context,
    ) -> Size {
        self.size = constraints.biggest();
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        (self.paint_fn)(self.size, canvas, context);
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
