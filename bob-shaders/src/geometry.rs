pub struct UVRectangle {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub u1: f32,
    pub u2: f32,
    pub v1: f32,
    pub v2: f32,
}

impl UVRectangle {
    pub fn as_xyuv(&self) -> Vec<f32> {
        let xyuv = vec![
            self.x1, self.y1, self.u1, self.v1, //
            self.x2, self.y1, self.u2, self.v1, //
            self.x1, self.y2, self.u1, self.v2, //
            self.x2, self.y2, self.u2, self.v2,
        ];
        xyuv
    }
}
