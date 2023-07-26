use gl_thin::gl_fancy::{BoundBuffers, GPUState};

pub mod flat_color_shader;
pub mod geometry;
pub mod masked_solid_shader;
pub mod raw_texture_shader;
pub mod sun_phong_shader;

pub trait GeometryBuffer<AT, IT> {
    fn activate<'a>(&'a self, gpu_state: &'a mut GPUState) -> BoundBuffers<'a, AT, IT>;
    fn deactivate(&self, bound_buffers: BoundBuffers<AT, IT>);
}
