use femtovg::{renderer::OpenGl, Canvas};
use std::fmt::Debug;

use super::{box_constraints::BoxConstraints, context::Context, size::Size};

pub trait Widget: Debug {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: Context,
    ) -> Size;
    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: Context);
    fn size(&self) -> Size;
    fn pack(self) -> Box<dyn Widget>;
}
