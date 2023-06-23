use crate::gl_helper::{GLErrorWrapper, Program};
use gl::types::{GLint, GLuint};

pub struct FlatColorShader {
    pub program: Program,
    pub sul_matrix: GLuint,
}

impl FlatColorShader {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        const VERTEX_SHADER: &str = "
uniform mat4 matrix;

attribute vec3 position;
attribute vec3 color;

varying vec3 vColor;

void main() {
    gl_Position = matrix * vec4(position, 1.0) ;
    vColor = color;
}
            ";
        const FRAGMENT_SHADER: &str = "
varying vec3 vColor;

void main() {
    gl_FragColor = vec4(vColor, 1.0);
}
            ";
        let program = Program::compile(VERTEX_SHADER, FRAGMENT_SHADER)?;
        let sul_matrix = program.get_uniform_location("matrix")?;
        Ok(Self {
            program,
            sul_matrix,
        })
    }

    pub fn set_params(&self, matrix: &[f32; 16]) {
        self.program
            .set_mat4u(self.sul_matrix as GLint, matrix)
            .unwrap();
    }
}
