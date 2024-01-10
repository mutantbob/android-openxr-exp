use bob_shaders::raw_texture_shader::RawTextureShader;
use gl::types::GLfloat;
use gl_thin::gl_fancy::{ActiveTextureUnit, GPUState, VertexBufferBundle};
use gl_thin::gl_helper::{GLErrorWrapper, TextureWithTarget};
use gl_thin::linear::XrMatrix4x4f;

pub struct TexturedQuad {
    pub program: RawTextureShader,
    pub buffers: VertexBufferBundle<'static, GLfloat, u8>,
    pub texture: TextureWithTarget,
}

impl TexturedQuad {
    pub fn new(
        gpu_state: &mut GPUState,
        dx: f32,
        dy: f32,
        texture: TextureWithTarget,
    ) -> Result<Self, GLErrorWrapper> {
        let program = RawTextureShader::new(gl::TEXTURE_2D)?;

        program.shader.use_()?;

        let buffers = {
            let quad = vec![
                -dx, -dy, 0.0, 1.0, //
                dx, -dy, 1.0, 1.0, //
                -dx, dy, 0.0, 0.0, //
                dx, dy, 1.0, 0.0,
            ];

            static INDICES: [u8; 4] = [0, 1, 2, 3];
            VertexBufferBundle::<'static, GLfloat, u8>::new(
                gpu_state,
                quad.into(),
                (&INDICES).into(),
                4,
                &[
                    (program.shader_attribute_position_location, 2, 0),
                    (program.shader_attribute_texture_location, 2, 2),
                ],
            )?
        };

        let rval = Self {
            buffers,
            program,
            texture,
        };

        Ok(rval)
    }

    pub fn index_count(&self) -> usize {
        4
    }

    pub fn paint_quad(
        &self,
        matrix: &XrMatrix4x4f,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        let tunit = ActiveTextureUnit(0);

        self.program
            .set_params(matrix, &self.texture, tunit, gpu_state)?;

        let binding = self.buffers.bind(gpu_state)?;

        binding.draw_elements(gl::TRIANGLE_STRIP, self.buffers.index_count as _, 0)?;

        drop(binding);

        Ok(())
    }
}
