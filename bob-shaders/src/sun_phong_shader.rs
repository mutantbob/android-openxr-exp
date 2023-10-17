use crate::GeometryBuffer;
use gl::types::{GLint, GLsizei};
use gl_thin::gl_fancy::{BoundBuffers, GPUState};
use gl_thin::gl_helper::{GLBufferType, GLErrorWrapper, Program};
use gl_thin::linear::XrMatrix4x4f;

//

pub struct SunPhongShader {
    pub program: Program,
    pub sal_position: u32,
    pub sal_normal: u32,
    pub sul_m_matrix: u32,
    pub sul_pv_matrix: u32,
}

impl SunPhongShader {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let program = Program::compile(shader_v_src(), shader_f_src())?;

        let sal_position = program.get_attribute_location("a_position")?;
        let sal_normal = program.get_attribute_location("a_normal")?;

        let sul_m_matrix = program.get_uniform_location("m_matrix")?;
        let sul_pv_matrix = program.get_uniform_location("pv_matrix")?;

        log::debug!(
            "attribute, uniform locations {} {}  {} {}",
            sal_position,
            sal_normal,
            sul_m_matrix,
            sul_pv_matrix,
        );

        Ok(Self {
            program,
            sal_position,
            sal_normal,
            sul_m_matrix,
            sul_pv_matrix,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<AT, IT: GLBufferType>(
        &self,
        m_matrix: &XrMatrix4x4f,
        pv_matrix: &XrMatrix4x4f,
        sun_direction: &[f32; 3],
        color: &[f32; 3],
        buffers: &dyn GeometryBuffer<AT, IT>,
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.program.use_()?;

        self.set_parameters(m_matrix, pv_matrix, sun_direction, color)?;

        let bindings = buffers.activate(gpu_state);

        bindings.draw_elements(gl::TRIANGLES, n_indices, 0)?;

        // unbind

        buffers.deactivate(bindings);
        unsafe {
            gl::DisableVertexAttribArray(self.sal_normal);
            gl::DisableVertexAttribArray(self.sal_position);
        }

        Ok(())
    }

    pub fn set_parameters(
        &self,
        m_matrix: &XrMatrix4x4f,
        pv_matrix: &XrMatrix4x4f,
        sun_direction: &[f32; 3],
        color: &[f32; 3],
    ) -> Result<(), GLErrorWrapper> {
        self.set_m_matrix(m_matrix)?;
        self.set_pv_matrix(pv_matrix)?;

        self.set_sun_direction(sun_direction)?;
        self.set_color(color)?;
        Ok(())
    }

    fn set_color(&self, color: &[f32; 3]) -> Result<(), GLErrorWrapper> {
        self.program
            .set_uniform_3f("color", color[0], color[1], color[2])
    }

    fn set_sun_direction(&self, sun_direction: &[f32; 3]) -> Result<(), GLErrorWrapper> {
        self.program.set_uniform_3f(
            "sun_direction",
            sun_direction[0],
            sun_direction[1],
            sun_direction[2],
        )
    }

    pub fn rig_attribute_arrays<AT: GLBufferType, IT: GLBufferType>(
        &self,
        binding: &BoundBuffers<AT, IT>,
    ) -> Result<(), GLErrorWrapper> {
        self.program.use_()?;
        binding.rig_one_attribute_by_name::<AT>(&self.program, "a_position", 3, 6, 0)?;
        binding.rig_one_attribute_by_name::<AT>(&self.program, "a_normal", 3, 6, 3)?;
        // Renderer::rig_one_va(&self.program, "a_position", 3, 6, 0)?;
        // Renderer::rig_one_va(&self.program, "a_normal", 3, 6, 3)?;
        Ok(())
    }

    fn set_m_matrix(&self, projection_matrix: &XrMatrix4x4f) -> Result<(), GLErrorWrapper> {
        self.program
            .set_mat4u(self.sul_m_matrix as GLint, projection_matrix.slice())
    }

    fn set_pv_matrix(&self, projection_matrix: &XrMatrix4x4f) -> Result<(), GLErrorWrapper> {
        self.program
            .set_mat4u(self.sul_pv_matrix as GLint, projection_matrix.slice())
    }
}

fn shader_v_src() -> &'static str {
    "
attribute vec4 a_position;
attribute vec3 a_normal;

varying vec3 v_normal;

uniform mat4 m_matrix;
uniform mat4 pv_matrix;

void main()
{
    gl_Position = pv_matrix * m_matrix * a_position;
    v_normal = mat3(m_matrix) * a_normal;
}
"
}

fn shader_f_src() -> &'static str {
    "#ifdef GL_ES
precision highp float;
#endif
varying vec3 v_normal;
uniform vec3 sun_direction;
uniform vec3 color;
void main()
{{
    vec3 N = normalize(v_normal);
    vec3 SD = normalize(sun_direction);
    float ambient=0.1;

    float lum = ambient+max(0.0, dot(N,SD));
    gl_FragColor = vec4(color*lum, 1.0);
}}"
}
