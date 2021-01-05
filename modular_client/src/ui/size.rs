#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }

    #[inline]
    pub fn from_height(height: f32) -> Self {
        Size::new(f32::INFINITY, height)
    }

    #[inline]
    pub fn from_width(width: f32) -> Self {
        Size::new(width, f32::INFINITY)
    }

    #[inline]
    pub fn from_radius(radius: f32) -> Self {
        Size::new(radius * 2.0, radius * 2.0)
    }

    #[inline]
    pub fn square(dimension: f32) -> Self {
        Size::new(dimension, dimension)
    }
    
    #[inline]
    pub fn infinite() -> Self {
        Size {
            width: f32::INFINITY,
            height: f32::INFINITY,
        }
    }

    #[inline]
    pub fn zero() -> Self {
        Size {
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height != 0.0 {
            self.width / self.height
        } else if self.width > 0.0 {
            f32::INFINITY
        } else if self.width < 0.0 {
            f32::NEG_INFINITY
        } else {
            0.0
        }
    }

    #[inline]
    pub fn flipped(&self) -> Self {
        Size::new(self.height, self.width)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    #[inline]
    pub fn is_infinite(&self) -> bool {
        self.width.is_infinite() && self.height.is_infinite()
    }
}
