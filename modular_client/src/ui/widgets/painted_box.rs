use femtovg::{renderer::OpenGl, Canvas, Color, Paint, Path};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug)]
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
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
    pub fn package(self) -> Box<Self> {
        Box::new(self)
    }
}

impl Widget for PaintedBox {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        self.size = constraints.biggest();
        if let Some(ref mut child) = self.child {
            child.layout(constraints, canvas, context);
        }
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        canvas.save_with(|canvas| {
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
