use core::fmt;

use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{context::Context, size::Size};

pub trait PaintFnTr: Fn(Size, &mut Canvas<OpenGl>, &Context) -> () {}
impl<F> PaintFnTr for F where F: Fn(Size, &mut Canvas<OpenGl>, &Context) -> () {}

// When you try to use PaintFnTr directly as a trait object, rust will complain that
// the associated type `Fn::Output` is not specified. This is a workaround:
/// Type alias for PaintFnTr trait objects.
pub type PaintFn = dyn PaintFnTr<Output = ()>;

impl fmt::Debug for PaintFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PaintFn")
    }
}
