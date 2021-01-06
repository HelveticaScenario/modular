use femtovg::{renderer::OpenGl, Canvas, Paint};

use crate::ui::{box_constraints::BoxConstraints, size::Size, widget::Widget};

pub struct Text {
    pub text: String,
    pub fill_paint: Option<Paint>,
    pub stroke_paint: Option<Paint>,
    pub size: Size,
}

impl Text {
    pub fn new(text: String) -> Self {
        Text {
            text,
            fill_paint: None,
            stroke_paint: None,
            size: Size::zero(),
        }
    }

    pub fn with_fill(mut self, fill: Paint) -> Self {
        self.fill_paint = Some(fill);
        self
    }

    pub fn with_stroke(mut self, stroke: Paint) -> Self {
        self.stroke_paint = Some(stroke);
        self
    }

    pub fn package(self) -> Box<Self> {
        Box::new(self)
    }

    fn get_size(&self, paint: Paint, canvas: &mut Canvas<OpenGl>) -> Size {
        let metrics = canvas.measure_text(0.0, 0.0, &self.text, paint).unwrap();
        Size::new(metrics.width(), metrics.height())
    }
}

impl Widget for Text {
    fn layout(&mut self, _constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        let stroke_size = if let Some(paint) = self.stroke_paint {
            self.get_size(paint, canvas)
        } else {
            Size::new(0.0, 0.0)
        };
        let fill_size = if let Some(paint) = self.fill_paint {
            self.get_size(paint, canvas)
        } else {
            Size::new(0.0, 0.0)
        };
        let size = Size::new(
            stroke_size.width.max(fill_size.width),
            stroke_size.height.max(fill_size.height),
        );

        self.size = size;
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        if let Some(paint) = self.fill_paint {
            canvas.fill_text(0.0, self.size.height, &self.text, paint).unwrap();
        }
        if let Some(paint) = self.stroke_paint {
            canvas.stroke_text(0.0, self.size.height, &self.text, paint).unwrap();
        };
    }

    fn size(&self) -> Size {
        self.size
    }
}
