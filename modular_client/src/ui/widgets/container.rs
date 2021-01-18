use femtovg::{Canvas, Color, renderer::OpenGl};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};

#[derive(Debug, Default)]
pub struct Container {
    pub size: Size,
    pub child: Option<Box<dyn Widget>>,
    pub color: Option<Color>,
}


impl Container {
    pub fn new(size: Size, child: Option<Box<dyn Widget>>, color: Option<Color>) -> Self {
        Container { size, child, color }
    }
}

impl Widget for Container {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: &Context,
    ) -> Size {
        if let Some(ref mut child) = self.child {
            child.layout(BoxConstraints::loose(self.size), canvas, context);
        }
        Size::new(
            constraints.max_width.min(self.size.width),
            constraints.max_height.min(self.size.height),
        )
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: &Context) {
        canvas.save_with(|canvas| {
            if let Some(color) = self.color {
                canvas.clear_rect(
                    0,
                    0,
                    self.size.width.round().max(0.0) as u32,
                    self.size.height.round().max(0.0) as u32,
                    color,
                );
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
