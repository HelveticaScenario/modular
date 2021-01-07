use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{
    box_constraints::BoxConstraints,
    context::Context,
    offset::{self, Offset},
    size::Size,
    widget::Widget,
};

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Default)]
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

#[derive(Debug)]
pub struct Align {
    pub child: Box<dyn Widget>,
    pub alignment: Alignment,
    pub size: Size,
}

impl Align {
    pub fn new(alignment: Alignment, child: impl Widget + 'static) -> Self {
        Align {
            child: Box::new(child),
            alignment,
            size: Size::zero(),
        }
    }
}

impl Widget for Align {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: Context,
    ) -> Size {
        self.child.layout(constraints, canvas, context);
        let size = constraints.biggest();
        self.size = size;
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: Context) {
        let child_size = self.child.size();
        let offset = self.alignment.to_offset(self.size) - self.alignment.to_offset(child_size);
        canvas.save_with(|canvas| {
            canvas.translate(offset.dx, offset.dy);
            self.child.paint(canvas, context);
        });
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
