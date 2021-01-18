use femtovg::{Canvas, Paint, Path, renderer::OpenGl};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget, widgets::custom_paint::CustomPaint};

#[derive(Debug)]
pub struct Grid {
    pub custom_paint: CustomPaint,
}

impl Grid {
    pub fn new(
        x_dim: f32,
        y_dim: f32,
        dash_on_ratio: f32,
        dash_off_ratio: f32,
        x_count: u32,
        y_count: u32,
    ) -> Self {
        let paint_grid = move |size: Size, canvas: &mut Canvas<OpenGl>, context: &Context| {
            {
                let mut path = Path::new();
                let dash_on =
                    x_dim * (dash_on_ratio / (dash_on_ratio + dash_off_ratio)) / x_count as f32;
                let dash_off =
                    x_dim * (dash_off_ratio / (dash_on_ratio + dash_off_ratio)) / x_count as f32;
                let mut y = -(dash_on / 2.0);
                while y < size.height {
                    path.move_to(0.0, y.max(0.0));
                    y += dash_on;
                    y = y.min(size.height);
                    path.line_to(0.0, y.max(0.0));
                    y += dash_off;
                }
                for i in 1..(size.width / x_dim).ceil() as i32 {
                    canvas.save_with(|canvas| {
                        canvas.translate(i as f32 * x_dim, 0.0);
                        let mut paint = Paint::color(context.theme.b_low);
                        paint.set_line_width(3.0);
                        canvas.stroke_path(&mut path, paint);
                    })
                }
            }
            {
                let mut path = Path::new();
                let dash_on =
                    y_dim * (dash_on_ratio / (dash_on_ratio + dash_off_ratio)) / y_count as f32;
                let dash_off =
                    y_dim * (dash_off_ratio / (dash_on_ratio + dash_off_ratio)) / y_count as f32;
                let mut x = -(dash_on / 2.0);
                while x < size.width {
                    path.move_to(x.max(0.0), 0.0);
                    x += dash_on;
                    x = x.min(size.width);
                    path.line_to(x.max(0.0), 0.0);
                    x += dash_off;
                }
                for i in 1..(size.height / y_dim).ceil() as i32 {
                    canvas.save_with(|canvas| {
                        canvas.translate(0.0, i as f32 * y_dim);
                        let mut paint = Paint::color(context.theme.b_low);

                        paint.set_line_width(3.0);
                        canvas.stroke_path(&mut path, paint);
                    })
                }
            }
        };

        Grid {
            custom_paint: CustomPaint::new(paint_grid),
        }
    }
}


impl Widget for Grid {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        self.custom_paint.layout(constraints, canvas, context)
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        self.custom_paint.paint(canvas, context)
    }

    fn size(&self) -> Size {
        self.custom_paint.size()
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}