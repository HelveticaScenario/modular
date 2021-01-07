use femtovg::{Canvas, renderer::OpenGl};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct SizedBox {
    pub child: Option<Box<dyn Widget>>,
    size: Size,
}

impl SizedBox {
    pub fn new() -> Self {
        Self {
            child: None,
            size: Size::zero(),
        }
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
        context: Context,
    ) -> Size {
        todo!()
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: Context) {
        todo!()
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
