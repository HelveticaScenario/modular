use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Offset {
    pub dx: f32,
    pub dy: f32,
}

impl Offset {
    pub fn new(dx: f32, dy: f32) -> Self {
        Offset {
            dx,
            dy
        }
    }
}


impl Add for Offset {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            self.dx + other.dx,
            self.dy + other.dy,
        )
    }
}

impl Sub for Offset {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(
            self.dx - other.dx,
            self.dy - other.dy,
        )
    }
}

impl Div for Offset {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self::new(
            self.dx / other.dx,
            self.dy / other.dy,
        )
    }
}

impl Mul for Offset {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self::new(
            self.dx * other.dx,
            self.dy * other.dy,
        )
    }
}
