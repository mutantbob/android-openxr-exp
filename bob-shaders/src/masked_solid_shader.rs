use crate::GeometryBuffer;
use gl::types::{GLenum, GLint, GLsizei};
use gl_thin::gl_fancy::GPUState;
use gl_thin::gl_helper::{explode_if_gl_error, GLBufferType, GLErrorWrapper, Program, Texture};
use gl_thin::linear::XrMatrix4x4f;
use log::debug;

pub struct MaskedSolidShader {
    pub program: Program,
    pub sal_position: u32,
    pub sal_tex_coord: u32,
    pub sul_matrix: u32,
    pub sul_tex: u32,
}

impl MaskedSolidShader {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let program = Program::compile(shader_v_src(), shader_f_src())?;

        let sal_position = program.get_attribute_location("a_position")?;
        let sal_tex_coord = program.get_attribute_location("a_texCoord")?;

        let sul_matrix = program.get_uniform_location("u_matrix")?;
        let sul_tex = program.get_uniform_location("tex")?;

        debug!(
            "attribute, uniform locations {} {}  {} {} ",
            sal_position, sal_tex_coord, sul_matrix, sul_tex,
        );

        Ok(Self {
            program,
            sal_position,
            sal_tex_coord,
            sul_matrix,
            sul_tex,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<AT, IT: GLBufferType>(
        &self,
        matrix: &XrMatrix4x4f,
        mask: &Texture,
        color: &[f32; 3],
        draw_mode: GLenum,
        buffers: &dyn GeometryBuffer<AT, IT>,
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.program.use_()?;

        self.set_u_matrix(matrix)?;

        self.set_color(color)?;

        let texture_image_unit = 0;
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + texture_image_unit);
        }
        explode_if_gl_error()?;
        mask.bind(gl::TEXTURE_2D)?;

        self.set_texture(texture_image_unit)?;

        let bindings = buffers.activate(gpu_state);

        bindings.draw_elements(draw_mode, n_indices, 0)?;

        // unbind

        buffers.deactivate(bindings);
        unsafe {
            gl::DisableVertexAttribArray(self.sal_tex_coord);
            gl::DisableVertexAttribArray(self.sal_position);
        }

        Ok(())
    }

    fn set_texture(&self, texture_unit: u32) -> Result<(), GLErrorWrapper> {
        self.program.set_uniform_1i("tex", texture_unit as GLint)
    }

    fn set_color(&self, color: &[f32; 3]) -> Result<(), GLErrorWrapper> {
        self.program
            .set_uniform_3f("color", color[0], color[1], color[2])
    }

    fn set_u_matrix(&self, matrix: &XrMatrix4x4f) -> Result<(), GLErrorWrapper> {
        self.program.set_mat4u(self.sul_matrix as GLint, matrix)
    }
}

fn shader_v_src() -> &'static str {
    "
attribute vec4 a_position;
attribute vec2 a_texCoord;

varying vec2 v_texCoord;

uniform mat4 u_matrix;

void main()
{
    gl_Position = u_matrix * a_position;
    v_texCoord = a_texCoord;
}
"
}

fn shader_f_src() -> &'static str {
    "#ifdef GL_ES
precision highp float;
#endif
varying vec2 v_texCoord;
uniform sampler2D tex;
uniform vec3 color;
void main()
{{
    float alpha = texture2D(tex, v_texCoord).r;
    gl_FragColor = vec4(color, alpha);

}}"
}
