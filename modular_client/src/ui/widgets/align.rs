use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, size::Size, widget::Widget};

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Alignment(f32, f32);

impl Alignment {
    pub fn bottom_center() -> Self {
        Alignment(0.0, 1.0)
    }

    pub fn bottom_left() -> Self {
        Alignment(-1.0, 1.0)
    }

    pub fn bottom_right() -> Self {
        Alignment(1.0, 1.0)
    }

    pub fn center() -> Self {
        Alignment(0.0, 0.0)
    }

    pub fn center_left() -> Self {
        Alignment(-1.0, 0.0)
    }

    pub fn center_right() -> Self {
        Alignment(1.0, 0.0)
    }

    pub fn top_center() -> Self {
        Alignment(0.0, -1.0)
    }

    pub fn top_left() -> Self {
        Alignment(-1.0, -1.0)
    }

    pub fn top_right() -> Self {
        Alignment(1.0, -1.0)
    }
}

pub struct Align {
    pub child: Box<dyn Widget>,
    pub alignment: Alignment,
    pub size: Option<Size>,
}

impl Align {
    pub fn new(child: Box<dyn Widget>, alignment: Alignment) -> Box<Self> {
        Box::new(Align {
            child,
            alignment,
            size: None,
        })
    }
}

impl Widget for Align {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        self.child.layout(constraints, canvas);
        let size = Size::new(constraints.max_width, constraints.max_height);
        self.size = Some(size);
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        let child_size = self.child.size();
        let half_child_width = child_size.width / 2.0;
        let half_child_height = child_size.height / 2.0;
        let Alignment(dx, dy) = self.alignment;
        let off_x = dx * half_child_width + half_child_width;
        let off_y = dy * half_child_height + half_child_height;
        canvas.save_with(|canvas| {
            let self_size = self.size.as_ref().unwrap();
            canvas.scissor(0.0, 0.0, self_size.width, self_size.height);
            canvas.translate(off_x, off_y);
            self.child.paint(canvas);
        });
    }

    fn size(&self) -> &Size {
        self.size.as_ref().unwrap()
    }
}
