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