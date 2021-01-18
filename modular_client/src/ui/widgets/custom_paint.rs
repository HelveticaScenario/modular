use std::fmt;

use femtovg::{Canvas, renderer::OpenGl};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

pub trait PaintFnTr: Fn(Size, &mut Canvas<OpenGl>, &Context) -> () { }
impl<F> PaintFnTr for F where F: Fn(Size, &mut Canvas<OpenGl>, &Context) -> () { }

// When you try to use PaintFnTr directly as a trait object, rust will complain that
// the associated type `Fn::Output` is not specified. This is a workaround:
/// Type alias for PaintFnTr trait objects.
pub type PaintFn = dyn PaintFnTr<Output = ()>;

impl fmt::Debug for PaintFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    { write!(f, "PaintFn") }
}

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