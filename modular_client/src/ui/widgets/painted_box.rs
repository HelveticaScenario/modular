use femtovg::{Color, Paint, Path};

use crate::ui::{size::Size, widget::Widget};

pub struct PaintedBox {
    pub stroke: Option<Paint>,
    pub fill: Option<Paint>,
    pub child: Option<Box<dyn Widget>>,
    size: Size,
}

impl PaintedBox {
    pub fn new() -> Self {
        PaintedBox {
            stroke: None,
            fill: None,
            child: None,
            size: Size::zero(),
        }
    }

    pub fn with_stroke(mut self, stroke: Paint) -> Self {
        self.stroke = Some(stroke);
        self
    }
    pub fn with_fill(mut self, fill: Paint) -> Self {
        self.fill = Some(fill);
        self
    }
    pub fn with_child(mut self, child: Box<dyn Widget>) -> Self {
        self.child = Some(child);
        self
    }
    pub fn package(self) -> Box<Self> {
        Box::new(self)
    }
}

impl Widget for PaintedBox {
    fn layout(
        &mut self,
        constraints: &crate::ui::box_constraints::BoxConstraints,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
    ) -> Size {
        self.size = constraints.biggest();
        if let Some(ref mut child) = self.child {
            child.layout(&constraints, canvas);
        }
        self.size
    }

    fn paint(&mut self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) {
        canvas.save_with(|canvas| {
            canvas.scissor(0.0, 0.0, self.size.width, self.size.height);
            if let Some(paint) = self.fill {
                let mut path = Path::new();
                path.rect(0.0, 0.0, self.size.width, self.size.height);
                canvas.fill_path(&mut path, paint);
            }
            if let Some(paint) = self.stroke {
                let mut path = Path::new();
                path.rect(0.0, 0.0, self.size.width, self.size.height);
                canvas.stroke_path(&mut path, paint);
            }
            if let Some(ref mut child) = self.child {
                child.paint(canvas);
            }
        })
    }

    fn size(&self) -> Size {
        self.size
    }
}
