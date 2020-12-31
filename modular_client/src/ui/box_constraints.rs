use super::size::Size;

pub struct BoxConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl BoxConstraints {
    pub fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        BoxConstraints {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    pub fn expand(width: Option<f32>, height: Option<f32>) -> Self {
        let width = if let Some(width) = width {
            width
        } else {
            f32::INFINITY
        };
        let height = if let Some(height) = height {
            height
        } else {
            f32::INFINITY
        };
        BoxConstraints {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    pub fn tight(size: Size) -> Self {
        BoxConstraints {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }
    pub fn loose(size: Size) -> Self {
        BoxConstraints {
            min_width: 0.0,
            max_width: size.width,
            min_height: 0.0,
            max_height: size.height,
        }
    }
}
