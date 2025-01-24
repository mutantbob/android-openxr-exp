use crate::text_painting;
use bob_shaders::flat_color_shader::FlatColorShader;
use bob_shaders::masked_solid_shader::MaskedSolidShader;
use bob_shaders::sun_phong_shader::SunPhongShader;
use bob_shaders::GeometryBuffer;
use gl::types::{GLfloat, GLint, GLsizei, GLushort};
use gl_thin::gl_fancy::{BoundBuffers, GPUState, VertexBufferBundle};
use gl_thin::gl_helper::{self, explode_if_gl_error, GLErrorWrapper, Program, TextureWithTarget};
use gl_thin::linear::XrMatrix4x4f;
use std::mem::size_of;

//

pub struct RainbowTriangle<'a> {
    pub program: FlatColorShader,
    pub buffers: VertexBufferBundle<'a, GLfloat, u8>,
}

impl RainbowTriangle<'_> {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        let program = FlatColorShader::new()?;

        program.program.use_()?;

        let buffers = {
            const COLOR_TRIANGLE: [GLfloat; 3 * 5] = [
                -0.5, -0.5, 0.0, 1.0, 0.0, //
                0.0, 0.5, 0.0, 0.0, 1.0, //
                0.5, -0.5, 1.0, 0.0, 0.0,
            ];

            static INDICES: [u8; 3] = [0, 1, 2];
            VertexBufferBundle::<'static, GLfloat, u8>::new(
                gpu_state,
                (&COLOR_TRIANGLE).into(),
                (&INDICES).into(),
                5,
                &[(program.sal_position, 2, 0), (program.sal_color, 3, 2)],
            )?
        };

        let rval = RainbowTriangle { buffers, program };

        Ok(rval)
    }

    pub fn paint_color_triangle(
        &self,
        matrix: &XrMatrix4x4f,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        let program = &self.program.program;
        program.use_().unwrap();

        self.program.set_params(matrix);

        let binding = self.buffers.bind(gpu_state)?;

        binding.draw_elements(gl::TRIANGLES, self.buffers.index_count as _, 0)?;

        drop(binding);

        Ok(())
    }

    #[deprecated]
    pub fn rig_one_va(
        program: &Program,
        name: &str,
        size: GLint,
        stride: GLsizei,
        offset: GLsizei,
    ) -> Result<(), GLErrorWrapper> {
        let loc = program.get_attribute_location(name)?;
        unsafe {
            gl::VertexAttribPointer(
                loc,
                size,
                gl::FLOAT,
                gl::FALSE,
                stride * size_of::<GLfloat>() as GLsizei,
                gl_helper::gl_offset_for::<GLfloat>(offset),
            );
        }
        explode_if_gl_error()?;
        unsafe {
            gl::EnableVertexAttribArray(loc);
        }
        explode_if_gl_error()
    }
}

//

pub struct Suzanne {
    phong: SunPhongShader,
    buffers: VertexBufferBundle<'static, GLfloat, GLushort>,
}

impl Suzanne {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        let phong = SunPhongShader::new()?;

        let indices = &crate::suzanne::TRIANGLE_INDICES;
        let buffers = VertexBufferBundle::new(
            gpu_state,
            (&crate::suzanne::XYZABC).into(),
            (indices).into(),
            6,
            &[(phong.sal_position, 3, 0), (phong.sal_normal, 3, 3)],
        )?;

        Ok(Self { phong, buffers })
    }

    pub fn index_count(&self) -> GLsizei {
        self.buffers.index_count as GLsizei
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        m_matrix: &XrMatrix4x4f,
        pv_matrix: &XrMatrix4x4f,
        sun_direction: &[f32; 3],
        color: &[f32; 3],
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.phong.draw(
            m_matrix,
            pv_matrix,
            sun_direction,
            color,
            self,
            n_indices,
            gpu_state,
        )
    }
}

impl GeometryBuffer<GLfloat, GLushort> for Suzanne {
    fn activate<'a>(&'a self, gpu_state: &'a mut GPUState) -> BoundBuffers<'a, GLfloat, GLushort> {
        self.buffers.bind(gpu_state).unwrap()
    }

    fn deactivate(&self, _droppable: BoundBuffers<GLfloat, GLushort>) {}
}

//

pub struct TextMessage {
    program: MaskedSolidShader,
    buffers: VertexBufferBundle<'static, GLfloat, GLushort>,
    texture: TextureWithTarget,
}

impl TextMessage {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        let tex_width = 256;
        let tex_height = 64;
        let aspect = tex_width as f32 / tex_height as f32;

        let xmin: f32 = -aspect;
        const YMIN: f32 = -1.0;
        let xmax: f32 = aspect;
        const YMAX: f32 = 1.0;
        const Z: f32 = 0.0;
        const UMIN: f32 = 0.0;
        const UMAX: f32 = 1.0;
        let xyuv = vec![
            xmin, YMIN, Z, UMIN, UMAX, //
            xmax, YMIN, Z, UMAX, UMAX, //
            xmin, YMAX, Z, UMIN, UMIN, //
            xmax, YMAX, Z, UMAX, UMIN, //
        ];
        let indices = &[0, 1, 2, 3];

        let program = MaskedSolidShader::new()?;

        let buffers = VertexBufferBundle::new(
            gpu_state,
            xyuv.into(),
            indices.into(),
            3 + 2,
            &[(program.sal_position, 3, 0), (program.sal_tex_coord, 2, 3)],
        )?;

        let texture = text_painting::text_to_greyscale_texture(
            tex_width,
            tex_height,
            66.0,
            "Hail Bob!",
            gpu_state,
            gl::TEXTURE_2D,
        )?;

        let rval = Self {
            program,
            buffers,
            texture,
        };
        Ok(rval)
    }

    pub fn index_count(&self) -> GLsizei {
        self.buffers.index_count as _
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        matrix: &XrMatrix4x4f,
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.program.draw(
            matrix,
            &self.texture,
            &[1.0, 0.5, 0.0, 1.0],
            None,
            gl::TRIANGLE_STRIP,
            self,
            n_indices,
            gpu_state,
        )
    }
}

impl GeometryBuffer<GLfloat, GLushort> for TextMessage {
    fn activate<'a>(&'a self, gpu_state: &'a mut GPUState) -> BoundBuffers<'a, GLfloat, GLushort> {
        self.buffers.bind(gpu_state).unwrap()
    }

    fn deactivate(&self, _droppable: BoundBuffers<GLfloat, GLushort>) {}
}
