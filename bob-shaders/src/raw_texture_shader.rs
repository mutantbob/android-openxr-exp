use gl::types::{GLfloat, GLint, GLsizei, GLuint};
use gl_thin::gl_fancy::{ActiveTextureUnit, BoundBuffers, GPUState};
use gl_thin::gl_helper::{gl_offset_for, GLBufferType, GLErrorWrapper, Program, TextureWithTarget};
use gl_thin::linear::XrMatrix4x4f;
use std::mem::size_of;

pub struct RawTextureShader {
    pub shader: Program,
    pub shader_attribute_position_location: u32,
    pub shader_attribute_texture_location: u32,
    pub sul_matrix: GLint,
}

impl Drop for RawTextureShader {
    fn drop(&mut self) {}
}

impl RawTextureShader {
    pub fn new(texture_target: GLuint) -> Result<RawTextureShader, GLErrorWrapper> {
        let shader = Program::compile(shader_v_src(), shader_f_src(texture_target))?;

        let shader_attribute_position_location =
            shader.get_attribute_location("a_position")? as u32;
        let shader_attribute_texture_location = shader.get_attribute_location("a_texcoord")? as u32;

        let sul_matrix = shader.get_uniform_location("u_matrix")? as GLint;

        Ok(RawTextureShader {
            shader,
            shader_attribute_position_location,
            shader_attribute_texture_location,
            sul_matrix,
        })
    }

    pub fn set_params(
        &self,
        matrix: &XrMatrix4x4f,
        texture: &TextureWithTarget,
        texture_image_unit: ActiveTextureUnit,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.shader.use_()?;
        gpu_state.set_active_texture(texture_image_unit)?;
        texture.bind()?;
        self.set_texture(texture_image_unit)?;
        self.set_u_matrix(matrix)
    }

    fn set_u_matrix(&self, matrix: &XrMatrix4x4f) -> Result<(), GLErrorWrapper> {
        self.shader.set_mat4u(self.sul_matrix, matrix.slice())
    }

    fn set_texture(&self, texture_unit: ActiveTextureUnit) -> Result<(), GLErrorWrapper> {
        self.shader.set_uniform_1i(
            self.shader.get_uniform_location("tex")? as _,
            texture_unit.0 as i32,
        )
    }

    pub fn draw<AT, IT: GLBufferType>(
        &self,
        gl_ram: &BoundBuffers<AT, IT>,
        indices_count: GLsizei,
        // indices: &[u16], xyzuvs: &[GLfloat]
    ) -> Result<(), GLErrorWrapper> {
        // gl_ram.fill_buffers(indices, xyzuvs);

        unsafe {
            gl::VertexAttribPointer(
                self.shader_attribute_position_location,
                3,
                gl::FLOAT,
                gl::FALSE,
                5 * size_of::<GLfloat>() as GLsizei,
                gl_offset_for::<AT>(0),
            );
            gl::VertexAttribPointer(
                self.shader_attribute_texture_location,
                2,
                gl::FLOAT,
                gl::FALSE,
                5 * size_of::<GLfloat>() as GLsizei,
                gl_offset_for::<AT>(3),
            );

            gl::EnableVertexAttribArray(self.shader_attribute_position_location);
            gl::EnableVertexAttribArray(self.shader_attribute_texture_location);
        }
        gl_ram.draw_elements(gl::TRIANGLES, indices_count, 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw2<AT, IT: GLBufferType>(
        &self,
        matrix: &XrMatrix4x4f,
        texture: &TextureWithTarget,
        texture_image_unit: ActiveTextureUnit,
        gl_ram: &BoundBuffers<AT, IT>,
        indices_count: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.set_params(matrix, texture, texture_image_unit, gpu_state)?;

        self.draw(gl_ram, indices_count)
    }
}

/*pub const IDENTITY: XrMatrix4x4f = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];*/

fn shader_v_src() -> &'static str {
    "
attribute vec4 a_position;
attribute vec2 a_texcoord;
varying vec2 v_texcoord;
uniform mat4 u_matrix;
void main()
{
    gl_Position = u_matrix * a_position;
    v_texcoord = a_texcoord;
}
"
}

fn shader_f_src(texture_target: GLuint) -> String {
    let (extension_directive, sampler_type) = if texture_target != gl::TEXTURE_2D {
        (
            "#extension GL_OES_EGL_image_external : require\n",
            "samplerExternalOES",
        )
    } else {
        ("", "sampler2D")
    };

    format!(
        "{}
#ifdef GL_ES
precision highp float;
#endif
varying vec2 v_texcoord;
uniform {} tex;
void main()
{{
    gl_FragColor = texture2D(tex, v_texcoord);
}}",
        extension_directive, sampler_type
    )
}
