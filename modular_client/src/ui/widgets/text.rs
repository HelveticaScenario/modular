use femtovg::{renderer::OpenGl, Canvas, Color, Paint, Path, TextMetrics};

use crate::ui::{
    box_constraints::BoxConstraints, context::Context, offset::Offset, size::Size, widget::Widget,
};

#[derive(Debug)]
pub struct Text {
    pub text: String,
    pub fill_paint: Option<Paint>,
    pub stroke_paint: Option<Paint>,
    pub size: Size,
    fill_offset: Offset,
    stroke_offset: Offset,
}

impl Text {
    pub fn new(text: String) -> Self {
        Text {
            text,
            fill_paint: None,
            stroke_paint: None,
            size: Size::zero(),
            fill_offset: Offset::new(0.0, 0.0),
            stroke_offset: Offset::new(0.0, 0.0),
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

    fn get_metrics(&self, paint: Paint, canvas: &mut Canvas<OpenGl>) -> TextMetrics {
        canvas.measure_text(0.0, 0.0, &self.text, paint).unwrap()
    }
}

impl Widget for Text {
    fn layout(
        &mut self,
        _constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        let (stroke_size, stroke_offset) = if let Some(paint) = self.stroke_paint {
            let metrics = self.get_metrics(paint, canvas);
            (
                Size::new(metrics.width(), metrics.height()),
                Offset::new(metrics.x, metrics.y),
            )
        } else {
            (Size::new(0.0, 0.0), Offset::new(0.0, 0.0))
        };
        let (fill_size, fill_offset) = if let Some(paint) = self.fill_paint {
            let metrics = self.get_metrics(paint, canvas);
            (
                Size::new(metrics.width(), metrics.height()),
                Offset::new(metrics.x, metrics.y),
            )
        } else {
            (Size::new(0.0, 0.0), Offset::new(0.0, 0.0))
        };
        self.fill_offset = fill_offset;
        self.stroke_offset = stroke_offset;
        let size = Size::new(
            stroke_size.width.max(fill_size.width),
            stroke_size.height.max(fill_size.height),
        );

        self.size = size;
        size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        let mut path = Path::new();
        path.rect(0.0, 0.0, self.size.width, self.size.height);
        canvas.stroke_path(&mut path, Paint::color(Color::white()));
        if let Some(paint) = self.fill_paint {
            canvas
                .fill_text(self.fill_offset.dx, -self.fill_offset.dy, &self.text, paint)
                .unwrap();
        }
        if let Some(paint) = self.stroke_paint {
            canvas
                .stroke_text(
                    self.stroke_offset.dx,
                    -self.stroke_offset.dy,
                    &self.text,
                    paint,
                )
                .unwrap();
        };
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}
