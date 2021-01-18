use femtovg::{Canvas, renderer::OpenGl};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct ConstrainedBox {
    pub constraints: BoxConstraints,
    pub child: Option<Box<dyn Widget>>,
    size: Size,
}

impl ConstrainedBox {
    pub fn new(constraints: BoxConstraints) -> Self {
        Self {
            constraints,
            child: None,
            size: Size::zero()
        }
    }

    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
}

impl Widget for ConstrainedBox {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        todo!()
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        todo!()
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
