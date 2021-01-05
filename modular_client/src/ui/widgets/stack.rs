use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, size::Size, widget::Widget};

pub struct Stack {
    pub children: Vec<Box<dyn Widget>>,
    size: Option<Size>,
}

impl Stack {
    pub fn new(children: Vec<Box<dyn Widget>>) -> Box<Self> {
        Box::new(Stack {
            children,
            size: None,
        })
    }
}

impl Widget for Stack {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        for child in self.children.iter_mut() {
            child.layout(constraints, canvas);
        }
        let size = Size::new(constraints.max_width, constraints.max_height);
        self.size = Some(size);
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        canvas.save_with(|canvas| {
            let self_size = self.size.as_ref().unwrap();
            canvas.scissor(0.0, 0.0, self_size.width, self_size.height);
            for child in self.children.iter_mut() {
                child.paint(canvas);
            }
        })
    }

    fn size(&self) -> &Size {
        self.size.as_ref().unwrap()
    }
}
