use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct SizedBox {
    pub child: Option<Box<dyn Widget>>,
    pub size: Size,
}

impl SizedBox {
    pub fn new(size: Size) -> Self {
        Self { child: None, size }
    }

    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
}

impl Widget for SizedBox {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        if let Some(ref mut child) = self.child {
            child.layout(
                BoxConstraints::loose(self.size).enforce(constraints),
                canvas,
                context,
            );
        }
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        if let Some(ref mut child) = self.child {
            child.paint(canvas, context);
        }
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
