use femtovg::Color;

use crate::ui::{box_constraints::BoxConstraints, size::Size, widget::Widget};

pub struct Container {
    pub size: Size,
    pub child: Option<Box<dyn Widget>>,
    pub color: Option<Color>,
}

impl Container {
    pub fn new(size: Size, child: Option<Box<dyn Widget>>, color: Option<Color>) -> Box<Self> {
        Box::new(Container { size, child, color })
    }
}

impl Widget for Container {
    fn layout(
        &mut self,
        constraints: &crate::ui::box_constraints::BoxConstraints,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
    ) -> Size {
        if let Some(ref mut child) = self.child {
            child.layout(&BoxConstraints::loose(self.size), canvas);
        }
        Size::new(
            constraints.max_width.min(self.size.width),
            constraints.max_height.min(self.size.height),
        )
    }

    fn paint(&mut self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) {
        canvas.save_with(|canvas| {
            canvas.scissor(0.0, 0.0, self.size.width, self.size.height);
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
                child.paint(canvas);
            }
        })
    }

    fn size(&self) -> Size {
        self.size
    }
}
