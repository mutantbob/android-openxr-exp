pub struct UVRectangle {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_u: f32,
    pub max_u: f32,
    pub min_v: f32,
    pub max_v: f32,
}

impl UVRectangle {
    pub fn uv_rect(&self) -> Vec<f32> {
        let xyuv = vec![
            self.min_x, self.min_y, self.min_u, self.max_v, //
            self.max_x, self.min_y, self.max_u, self.max_v, //
            self.min_x, self.max_y, self.min_u, self.min_v, //
            self.max_x, self.max_y, self.max_u, self.min_v,
        ];
        xyuv
    }
}
