use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{
    box_constraints::BoxConstraints,
    offset::{self, Offset},
    size::Size,
    widget::Widget,
};

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

    pub fn to_offset(&self, size: Size) -> Offset {
        Offset::new(
            size.width * ((self.0 + 1.0) / 2.0),
            size.height * ((self.1 + 1.0) / 2.0),
        )
    }
}

pub struct Align {
    pub child: Box<dyn Widget>,
    pub alignment: Alignment,
    pub size: Size,
}

impl Align {
    pub fn new(alignment: Alignment, child: Box<dyn Widget>) -> Box<Self> {
        Box::new(Align {
            child,
            alignment,
            size: Size::zero(),
        })
    }
}

impl Widget for Align {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        self.child.layout(constraints, canvas);
        let size = constraints.biggest();
        self.size = size;
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        let child_size = self.child.size();
        let offset = self.alignment.to_offset(self.size) - self.alignment.to_offset(child_size);
        canvas.save_with(|canvas| {
            canvas.translate(offset.dx, offset.dy);
            canvas.scissor(0.0, 0.0, child_size.width, child_size.height);
            self.child.paint(canvas);
        });
    }

    fn size(&self) -> Size {
        self.size
    }
}
