use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
pub struct Stack {
    pub children: Vec<Box<dyn Widget>>,
    size: Size,
}

impl Stack {
    pub fn new(children: Vec<impl Widget + 'static>) -> Self {
        Stack {
            children: children.into_iter().map(|child| child.pack()).collect(),
            size: Size::zero(),
        }
    }
}

impl Widget for Stack {
    fn layout(&mut self, constraints: BoxConstraints, canvas: &mut Canvas<OpenGl>, context: &Context) -> Size {
        for child in self.children.iter_mut() {
            child.layout(constraints, canvas, context);
        }
        let size = Size::new(constraints.max_width, constraints.max_height);
        self.size = size;
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        canvas.save_with(|canvas| {
            for child in self.children.iter_mut() {
                child.paint(canvas, context);
            }
        })
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
