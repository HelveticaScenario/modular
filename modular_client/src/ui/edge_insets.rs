use std::ops::{Add, Div, Mul, Sub};

use modular_core::dsp::utils::clamp;

use super::{offset::Offset, rect::Rect, size::Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

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

    pub fn all(value: f32) -> Self {
        Self::from_ltrb(value, value, value, value)
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self::from_ltrb(horizontal, vertical, horizontal, vertical)
    }

    pub fn collapsed_size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn top_left(&self) -> Offset {
        Offset::new(self.left, self.top)
    }

    pub fn top_right(&self) -> Offset {
        Offset::new(-self.right, self.top)
    }

    pub fn bottom_left(&self) -> Offset {
        Offset::new(self.left, -self.bottom)
    }

    pub fn bottom_right(&self) -> Offset {
        Offset::new(-self.right, -self.bottom)
    }

    pub fn clamp(&self, min: Self, max: Self) -> Self {
        Self::from_ltrb(
            clamp(min.left, max.left, self.left),
            clamp(min.top, max.top, self.top),
            clamp(min.right, max.right, self.right),
            clamp(min.bottom, max.bottom, self.bottom),
        )
    }

    pub fn deflate_rect(&self, rect: Rect) -> Rect {
        Rect::from_ltrb(rect.left + self.left, rect.top + self.top, rect.right - self.right, rect.bottom - self.bottom)
    }

    pub fn deflate_size(&self, size: Size) -> Size {
        Size::new(size.width - self.horizontal(), size.height - self.vertical())
    }

    pub fn inflate_rect(&self, rect: Rect) -> Rect {
        Rect::from_ltrb(rect.left - self.left, rect.top - self.top, rect.right + self.right, rect.bottom + self.bottom)
    }

    pub fn inflate_size(&self, size: Size) -> Size {
        Size::new(size.width + self.horizontal(), size.height + self.vertical())
    }
}

impl Add for EdgeInsets {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::from_ltrb(
            self.left + other.left,
            self.top + other.top,
            self.right + other.right,
            self.bottom + other.bottom,
        )
    }
}

impl Sub for EdgeInsets {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::from_ltrb(
            self.left - other.left,
            self.top - other.top,
            self.right - other.right,
            self.bottom - other.bottom,
        )
    }
}

impl Div for EdgeInsets {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self::from_ltrb(
            self.left / other.left,
            self.top / other.top,
            self.right / other.right,
            self.bottom / other.bottom,
        )
    }
}

impl Mul for EdgeInsets {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self::from_ltrb(
            self.left * other.left,
            self.top * other.top,
            self.right * other.right,
            self.bottom * other.bottom,
        )
    }
}
