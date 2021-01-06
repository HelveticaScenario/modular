use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, size::Size, widget::Widget};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainAxisSize {
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerticalDirection {
    /// Boxes should start at the bottom and be stacked vertically towards the top.
    ///
    /// The "start" is at the bottom, the "end" is at the top.
    Up,

    /// Boxes should start at the top and be stacked vertically towards the bottom.
    ///
    /// The "start" is at the top, the "end" is at the bottom.
    Down,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// A direction along either the horizontal or vertical [Axis].
pub enum AxisDirection {
    /// Zero is at the bottom and positive values are above it: `⇈`
    Up,

    /// Zero is on the left and positive values are to the right of it: `⇉`
    Right,

    /// Zero is at the top and positive values are below it: `⇊`
    Down,

    /// Zero is to the right and positive values are to the left of it: `⇇`
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainAxisAlignment {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrossAxisAlignment {
    Start,
    End,
    Center,
    Stretch,
    Baseline,
}

pub struct Flex {
    pub children: Vec<Flexible>,
    pub direction: Axis,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,
    size: Size,
}

impl Flex {
    pub fn new(direction: Axis) -> Self {
        Flex {
            children: vec![],
            direction,
            main_axis_alignment: MainAxisAlignment::Start,
            main_axis_size: MainAxisSize::Max,
            cross_axis_alignment: CrossAxisAlignment::Center,
            vertical_direction: VerticalDirection::Down,
            size: Size::zero(),
        }
    }

    pub fn with_main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = main_axis_alignment;
        self
    }

    pub fn with_main_axis_size(mut self, main_axis_size: MainAxisSize) -> Self {
        self.main_axis_size = main_axis_size;
        self
    }

    pub fn with_cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    pub fn with_vertical_direction(mut self, vertical_direction: VerticalDirection) -> Self {
        self.vertical_direction = vertical_direction;
        self
    }

    pub fn with_child(mut self, child: Box<dyn Widget>) -> Self {
        self.children.push(Flexible::new(0.0, Some(child)));
        self
    }

    pub fn with_flex_child(mut self, flex: f32, child: Box<dyn Widget>) -> Self {
        self.children.push(Flexible::new(flex, Some(child)));
        self
    }

    pub fn with_spacer(mut self, flex: f32) -> Self {
        self.children.push(Flexible::new(flex, None));
        self
    }

    pub fn package(self) -> Box<Self> {
        Box::new(self)
    }
}

impl Widget for Flex {
    fn layout(&mut self, constraints: &BoxConstraints, canvas: &mut Canvas<OpenGl>) -> Size {
        let remainder = match self.direction {
            Axis::Horizontal => constraints.max_width,
            Axis::Vertical => constraints.max_height,
        };
        let direction = self.direction;
        let remainder = self
            .children
            .iter_mut()
            .filter(|Flexible { flex, child }| flex <= &0.0 && child.is_some())
            .fold(remainder, |accum, Flexible { child, .. }| {
                let size = child.as_mut().unwrap().layout(constraints, canvas);
                match direction {
                    Axis::Horizontal => accum - size.width,
                    Axis::Vertical => accum - size.height,
                }
            });
        let flex_sum: f32 = self.children.iter().map(|Flexible { flex, .. }| flex).sum();
        for Flexible { flex, child } in self
            .children
            .iter_mut()
            .filter(|Flexible { flex, child }| flex > &0.0 && child.is_some())
        {
            let dimension = remainder * (*flex / flex_sum);
            let constriants = match self.direction {
                Axis::Horizontal => {
                    
                }
                Axis::Vertical => {}
            };
        }
        // if has_flexible {
        //     for Flexible{flex, child} in self.children.iter_mut() {

        //     }
        // } else if  {

        //     for Flexible{flex, child} in self.children.iter_mut() {

        //     }
        // }
        self.size = constraints.biggest();
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>) {
        todo!()
    }

    fn size(&self) -> Size {
        todo!()
    }
}

pub struct Flexible {
    pub flex: f32,
    pub child: Option<Box<dyn Widget>>,
}

impl Flexible {
    pub fn new(flex: f32, child: Option<Box<dyn Widget>>) -> Self {
        Flexible { flex, child }
    }
}
