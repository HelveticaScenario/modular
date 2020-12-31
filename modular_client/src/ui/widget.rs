use femtovg::{Canvas, renderer::OpenGl};

use super::{box_constraints::BoxConstraints, size::Size};

pub trait Widget {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size;
    fn paint(&mut self, canvas: &mut Canvas<OpenGl>);
    fn size(&self) -> &Size;
}
