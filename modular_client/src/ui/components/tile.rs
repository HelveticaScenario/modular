use femtovg::{renderer::OpenGl, Canvas, Paint, Path};

use crate::ui::{
    box_constraints::BoxConstraints,
    context::Context,
    paint::{PaintFn, PaintFnTr},
    size::Size,
    widget::Widget,
    widgets::custom_paint::CustomPaint,
};

#[derive(Debug)]
pub struct Tile {
    pub custom_paint: CustomPaint,
}

impl Tile {
    pub fn new(tile_size: Size, paint_fn: impl PaintFnTr<Output = ()> + 'static) -> Self {
        let paint_fn = move |size: Size, canvas: &mut Canvas<OpenGl>, context: &Context| {
            for off_x in 0..(size.width / tile_size.width).ceil() as i32 {
                for off_y in 0..(size.height / tile_size.height).ceil() as i32 {
                    canvas.save_with(|canvas| {
                        canvas.translate(
                            off_x as f32 * tile_size.width,
                            off_y as f32 * tile_size.height,
                        );
                        paint_fn(tile_size, canvas, context);
                    });
                }
            }
        };
        Tile {
            custom_paint: CustomPaint::new(paint_fn),
        }
    }
}

impl Widget for Tile {
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
