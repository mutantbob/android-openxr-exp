use crate::GeometryBuffer;
use gl::types::{GLenum, GLint, GLsizei};
use gl_thin::gl_fancy::GPUState;
use gl_thin::gl_helper::{
    explode_if_gl_error, GLBufferType, GLErrorWrapper, Program, TextureWithTarget,
};
use gl_thin::linear::XrMatrix4x4f;
use log::debug;

/// uses the red channel of a texture as an alpha channel to mix a foreground and background color.
pub struct MaskedSolidShader {
    pub program: Program,
    pub sal_position: u32,
    pub sal_tex_coord: u32,
    pub sul_matrix: u32,
    pub sul_tex: u32,
    pub sul_color_fg: u32,
    pub sul_color_bg: u32,
}

impl MaskedSolidShader {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let program = Program::compile(shader_v_src(), shader_f_src())?;

        let sal_position = program.get_attribute_location("a_position")?;
        let sal_tex_coord = program.get_attribute_location("a_texCoord")?;

        let sul_matrix = program.get_uniform_location("u_matrix")?;
        let sul_tex = program.get_uniform_location("tex")?;
        let sul_color_fg = program.get_uniform_location("color_fg")?;
        let sul_color_bg = program.get_uniform_location("color_bg")?;

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
            sul_color_fg,
            sul_color_bg,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<AT, IT: GLBufferType>(
        &self,
        matrix: &XrMatrix4x4f,
        mask: &TextureWithTarget,
        color_fg: &[f32; 4],
        color_bg: Option<&[f32; 4]>,
        draw_mode: GLenum,
        buffers: &dyn GeometryBuffer<AT, IT>,
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.program.use_()?;

        let texture_image_unit = 0;
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + texture_image_unit);
        }
        explode_if_gl_error()?;
        mask.bind()?;

        self.set_parameters(
            texture_image_unit,
            color_fg,
            color_bg.unwrap_or(&[0.0; 4]),
            matrix,
        )?;

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

    pub fn set_parameters(
        &self,
        texture_unit: u32,
        color_fg: &[f32; 4],
        color_bg: &[f32; 4],
        matrix: &XrMatrix4x4f,
    ) -> Result<(), GLErrorWrapper> {
        self.set_texture(texture_unit)?;
        self.set_color_fg(color_fg)?;
        self.set_color_bg(color_bg)?;
        self.set_u_matrix(matrix)?;
        Ok(())
    }

    fn set_texture(&self, texture_unit: u32) -> Result<(), GLErrorWrapper> {
        self.program.set_uniform_1i(
            self.program.get_uniform_location("tex")? as _,
            texture_unit as GLint,
        )
    }

    fn set_color_fg(&self, color: &[f32; 4]) -> Result<(), GLErrorWrapper> {
        self.program.set_uniform_4f(
            self.sul_color_fg as GLint,
            color[0],
            color[1],
            color[2],
            color[3],
        )
    }

    fn set_color_bg(&self, color: &[f32; 4]) -> Result<(), GLErrorWrapper> {
        self.program.set_uniform_4f(
            self.sul_color_bg as GLint,
            color[0],
            color[1],
            color[2],
            color[3],
        )
    }

    fn set_u_matrix(&self, matrix: &XrMatrix4x4f) -> Result<(), GLErrorWrapper> {
        self.program
            .set_mat4u(self.sul_matrix as GLint, matrix.slice())
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
uniform vec4 color_fg;
uniform vec4 color_bg;
void main()
{{
    float alpha = texture2D(tex, v_texCoord).r;
    gl_FragColor = mix(color_bg, color_fg, alpha);
}}"
}
