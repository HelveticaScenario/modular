use crate::ui::{size::Size, widget::Widget};



pub struct Stack {
    pub children: Vec<Box<dyn Widget>>,
    size: Option<Size>
}

impl Stack {
    pub fn new(children: Vec<Box<dyn Widget>>) -> Self {
        Stack {
            children,
            size: None
        }
    }
}

impl Widget for Stack {
    fn layout(&mut self, constraints: &crate::ui::box_constraints::BoxConstraints, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) -> Size {
        todo!()
    }

    fn paint(&mut self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) {
        todo!()
    }

    fn size(&self) -> &Size {
        todo!()
    }
}