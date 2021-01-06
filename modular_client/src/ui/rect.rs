/// Ported from the Rect class from Flutter 1.22.5
use super::{offset::Offset, size::Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub fn new() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub fn with_left(mut self, left: f32) -> Self {
        self.left = left;
        self
    }

    pub fn with_top(mut self, top: f32) -> Self {
        self.top = top;
        self
    }

    pub fn with_right(mut self, right: f32) -> Self {
        self.right = right;
        self
    }

    pub fn with_bottom(mut self, bottom: f32) -> Self {
        self.bottom = bottom;
        self
    }

    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_center(center: Offset, size: Size) -> Self {
        Self::from_ltrb(
            center.dx - size.width / 2.0,
            center.dy - size.height / 2.0,
            center.dx + size.width / 2.0,
            center.dy + size.height / 2.0,
        )
    }

    pub fn from_circle(center: Offset, radius: f32) -> Self {
        Self::from_center(center, Size::new(radius * 2.0, radius * 2.0))
    }

    pub fn from_ltwh(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self::from_ltrb(left, top, left + width, top + height)
    }

    pub fn from_lt_size(left: f32, top: f32, size: Size) -> Self {
        Self::from_ltwh(left, top, size.width, size.height)
    }

    pub fn from_points(a: Offset, b: Offset) -> Self {
        Self::from_ltrb(
            a.dx.min(b.dx),
            a.dy.min(b.dy),
            a.dx.max(b.dx),
            a.dy.max(b.dy),
        )
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn bottom_center(&self) -> Offset {
        Offset::new(self.left + self.width() / 2.0, self.bottom)
    }

    pub fn bottom_left(&self) -> Offset {
        Offset::new(self.left, self.bottom)
    }

    pub fn bottom_right(&self) -> Offset {
        Offset::new(self.right, self.bottom)
    }

    pub fn center(&self) -> Offset {
        Offset::new(
            self.left + self.width() / 2.0,
            self.top + self.height() / 2.0,
        )
    }

    pub fn center_left(&self) -> Offset {
        Offset::new(self.left, self.top + self.height() / 2.0)
    }

    pub fn center_right(&self) -> Offset {
        Offset::new(self.right, self.top + self.height() / 2.0)
    }

    pub fn has_nan(&self) -> bool {
        self.left.is_nan() || self.top.is_nan() || self.right.is_nan() || self.bottom.is_nan()
    }

    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    pub fn is_finite(&self) -> bool {
        self.left.is_finite()
            || self.top.is_finite()
            || self.right.is_finite()
            || self.bottom.is_finite()
    }

    pub fn is_infinite(&self) -> bool {
        self.left >= f32::INFINITY
            || self.top >= f32::INFINITY
            || self.right >= f32::INFINITY
            || self.bottom >= f32::INFINITY
    }

    pub fn longest_side(&self) -> f32 {
        self.width().abs().max(self.height().abs())
    }

    pub fn shortest_side(&self) -> f32 {
        self.width().abs().min(self.height().abs())
    }

    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    pub fn top_center(&self) -> Offset {
        Offset::new(self.left + self.width() / 2.0, self.top)
    }

    pub fn top_left(&self) -> Offset {
        Offset::new(self.left, self.top)
    }

    pub fn top_right(&self) -> Offset {
        Offset::new(self.right, self.top)
    }

    pub fn contains(&self, offset: Offset) -> bool {
        offset.dx >= self.left
            && offset.dx < self.right
            && offset.dy >= self.top
            && offset.dy < self.bottom
    }

    pub fn deflate(&self, delta: f32) -> Self {
        self.inflate(-delta)
    }

    pub fn expand_to_include(&self, other: Rect) -> Self {
        Self::from_ltrb(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    pub fn inflate(&self, delta: f32) -> Self {
        Self::from_ltrb(
            self.left - delta,
            self.top - delta,
            self.right + delta,
            self.bottom + delta,
        )
    }

    pub fn intersect(&self, other: Rect) -> Self {
        Self::from_ltrb(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    pub fn overlaps(&self, other: Rect) -> bool {
        !(self.right <= other.left
            || other.right <= self.left
            || self.bottom <= other.top
            || other.bottom <= self.top)
    }

    pub fn shift(&self, offset: Offset) -> Self {
        Self::from_ltrb(
            self.left + offset.dx,
            self.top + offset.dy,
            self.right + offset.dx,
            self.bottom + offset.dy,
        )
    }

    /// From the flutter docs: This covers the space from -1e9,-1e9 to 1e9,1e9. This is the space over which graphics operations are valid.
    pub fn largest() -> Self {
        Self::from_ltrb(-1e9, -1e9, 1e9, 1e9)
    }
}
