use std::slice::IterMut;

use femtovg::{renderer::OpenGl, Canvas};

use crate::ui::{box_constraints::BoxConstraints, context::Context, size::Size, widget::Widget};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisSize {
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn max_constraint(&self, constraints: BoxConstraints) -> f32 {
        match self {
            Axis::Horizontal => constraints.max_width,
            Axis::Vertical => constraints.max_height,
        }
    }
    pub fn span(&self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    pub fn size(&self, span: f32) -> Size {
        match self {
            Axis::Horizontal => Size::new(span, f32::INFINITY),
            Axis::Vertical => Size::new(f32::INFINITY, span),
        }
    }
}
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum VerticalDirection {
//     /// Boxes should start at the bottom and be stacked vertically towards the top.
//     ///
//     /// The "start" is at the bottom, the "end" is at the top.
//     Up,

//     /// Boxes should start at the top and be stacked vertically towards the bottom.
//     ///
//     /// The "start" is at the top, the "end" is at the bottom.
//     Down,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisAlignment {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum CrossAxisAlignment {
//     Start,
//     End,
//     Center,
//     Stretch,
//     // Baseline, // I dont understand baselines right now, so i'll skip it
// }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexRule {
    Flex(f32),
    Fixed(f32),
}

#[derive(Debug)]
pub struct Flex {
    pub children: Vec<FlexChild>,
    pub direction: Axis,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
    // pub cross_axis_alignment: CrossAxisAlignment,
    // pub vertical_direction: VerticalDirection,
    size: Size,
}

impl Flex {
    pub fn new(direction: Axis) -> Self {
        Flex {
            children: vec![],
            direction,
            main_axis_alignment: MainAxisAlignment::Start,
            main_axis_size: MainAxisSize::Max,
            // cross_axis_alignment: CrossAxisAlignment::Center,
            // vertical_direction: VerticalDirection::Down,
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

    // pub fn with_cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Self {
    //     self.cross_axis_alignment = cross_axis_alignment;
    //     self
    // }

    // pub fn with_vertical_direction(mut self, vertical_direction: VerticalDirection) -> Self {
    //     self.vertical_direction = vertical_direction;
    //     self
    // }

    pub fn with_child(mut self, rule: FlexRule, child: impl Widget + 'static) -> Self {
        self.children
            .push(FlexChild::new(rule, Some(Box::new(child))));
        self
    }

    pub fn with_spacer(mut self, rule: FlexRule) -> Self {
        self.children.push(FlexChild::new(rule, None));
        self
    }

    pub fn paint_with_space(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        context: Context,
        space_between: f32,
    ) {
        for FlexChild {
            flex_rule,
            widget,
            span,
        } in self.children.iter_mut()
        {
            if let Some(widget) = widget {
                canvas.save_with(|canvas| {
                    widget.paint(canvas, context);
                });
            };
            Self::translate(self.direction, canvas, *span + space_between);
        }
    }

    pub fn translate(direction: Axis, canvas: &mut Canvas<OpenGl>, span: f32) {
        match direction {
            Axis::Horizontal => {
                canvas.translate(span, 0.0f32);
            }
            Axis::Vertical => {
                canvas.translate(0.0f32, span);
            }
        }
    }
}

impl Widget for Flex {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        canvas: &mut Canvas<OpenGl>,
        context: Context,
    ) -> Size {
        let fixed_space: f32 = self
            .children
            .iter()
            .map(|FlexChild { flex_rule, .. }| match flex_rule {
                FlexRule::Flex(_) => 0.0f32,
                FlexRule::Fixed(span) => *span,
            })
            .sum();
        let flex_sum: f32 = self
            .children
            .iter()
            .map(|FlexChild { flex_rule, .. }| match flex_rule {
                FlexRule::Flex(flex) => *flex,
                FlexRule::Fixed(_) => 0.0,
            })
            .sum();

        let flex_space = match self.main_axis_size {
            MainAxisSize::Min => 0.0f32,
            MainAxisSize::Max => self.direction.max_constraint(constraints) - fixed_space,
        };

        for flex_child in self.children.iter_mut() {
            let span = match flex_child.flex_rule {
                FlexRule::Flex(flex) => flex_space * (flex / flex_sum),
                FlexRule::Fixed(span) => span,
            };
            flex_child.span = span;
            if let Some(ref mut widget) = flex_child.widget {
                let constraints =
                    BoxConstraints::loose(self.direction.size(span)).enforce(constraints);
                widget.layout(constraints, canvas, context);
            }
        }
        self.size = match self.main_axis_size {
            MainAxisSize::Min => {
                BoxConstraints::loose(self.direction.size(fixed_space + flex_space))
                    .enforce(constraints)
                    .biggest()
            }
            MainAxisSize::Max => constraints.biggest(),
        };
        self.size
    }

    fn paint(&mut self, canvas: &mut Canvas<OpenGl>, context: Context) {
        let child_space: f32 = self
            .children
            .iter()
            .map(|FlexChild { span, .. }| *span)
            .sum();
        let full_space = self.direction.span(self.size);
        canvas.save_with(|canvas| {
            match self.main_axis_alignment {
                MainAxisAlignment::Start => {
                    self.paint_with_space(canvas, context, 0.0f32);
                }
                MainAxisAlignment::End => {
                    Self::translate(self.direction, canvas, full_space - child_space);
                    self.paint_with_space(canvas, context, 0.0);
                }
                MainAxisAlignment::Center => {
                    Self::translate(self.direction, canvas, (full_space - child_space) / 2.0);
                    self.paint_with_space(canvas, context, 0.0);
                }
                MainAxisAlignment::SpaceBetween => {
                    let space_count = self.children.len() - 1;
                    self.paint_with_space(
                        canvas,
                        context,
                        if space_count > 0 {
                            (full_space - child_space) / space_count as f32
                        } else {
                            0.0
                        },
                    );
                }
                MainAxisAlignment::SpaceAround => {
                    let space_count = self.children.len();
                    if space_count > 0 {
                        Self::translate(
                            self.direction,
                            canvas,
                            (full_space - child_space) / space_count as f32 / 2.0,
                        );
                    }
                    self.paint_with_space(
                        canvas,
                        context,
                        if space_count > 0 {
                            (full_space - child_space) / space_count as f32
                        } else {
                            0.0
                        },
                    );
                }
                MainAxisAlignment::SpaceEvenly => {
                    let space_count = self.children.len() + 1;

                    Self::translate(
                        self.direction,
                        canvas,
                        (full_space - child_space) / space_count as f32,
                    );
                    self.paint_with_space(
                        canvas,
                        context,
                        (full_space - child_space) / space_count as f32,
                    );
                }
            };
        });
    }

    fn size(&self) -> Size {
        self.size
    }

    fn pack(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

#[derive(Debug)]
pub struct FlexChild {
    pub flex_rule: FlexRule,
    pub widget: Option<Box<dyn Widget>>,
    span: f32,
}

impl FlexChild {
    pub fn new(flex_rule: FlexRule, widget: Option<Box<dyn Widget>>) -> Self {
        FlexChild {
            flex_rule,
            widget,
            span: 0.0,
        }
    }
}
