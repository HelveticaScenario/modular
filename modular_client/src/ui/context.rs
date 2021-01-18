use femtovg::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub f_high: Color,
    pub f_med: Color,
    pub f_low: Color,
    pub f_inv: Color,
    pub b_high: Color,
    pub b_med: Color,
    pub b_low: Color,
    pub b_inv: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Context {
    pub dpi_factor: f32,
    pub theme: Theme
}
