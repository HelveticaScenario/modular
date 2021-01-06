use modular_core::dsp::utils::clamp;

use super::{edge_insets::EdgeInsets, size::Size};

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn biggest(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    pub fn smallest(&self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    pub fn deflate(&self, edges: EdgeInsets) -> Self {
        let horizontal = edges.horizontal();
        let vertical = edges.vertical();
        let deflated_min_width = 0.0f32.max(self.min_width - horizontal);
        let deflated_min_height = 0.0f32.max(self.min_height - vertical);
        Self::new(
            deflated_min_width,
            deflated_min_width.max(self.max_width - horizontal),
            deflated_min_height,
            deflated_min_height.max(self.max_height - vertical),
        )
    }

    pub fn tighten(&self, width: Option<f32>, height: Option<f32>) -> Self {
        BoxConstraints::new(
            if let Some(width) = width {
                clamp(self.min_width, self.max_width, width)
            } else {
                self.min_width
            },
            if let Some(width) = width {
                clamp(self.min_width, self.max_width,width)
            } else {
                self.max_width
            },
            if let Some(height) = height {
                clamp(self.min_height, self.max_height,height)
            } else {
                self.min_height
            },
            if let Some(height) = height {
                clamp(self.min_height, self.max_height,height)
            } else {
                self.max_height
            },
        )
    }
}
