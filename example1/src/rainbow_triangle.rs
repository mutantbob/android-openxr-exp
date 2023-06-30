use crate::flat_color_shader::FlatColorShader;
use crate::raw_texture_shader::RawTextureShader;
use crate::sun_phong_shader::{GeometryBuffer, SunPhongShader};
use crate::text_painting;
use gl::types::{GLfloat, GLint, GLsizei, GLushort};
use gl_thin::gl_fancy::{BoundBuffers, BoundBuffersMut, GPUState, VertexBufferBundle};
use gl_thin::gl_helper::{
    self, explode_if_gl_error, GLBufferType, GLErrorWrapper, Program, Texture,
};
use gl_thin::linear::{
    xr_matrix4x4f_create_projection_fov, xr_matrix4x4f_create_scale,
    xr_matrix4x4f_create_translation, xr_matrix4x4f_create_translation_rotation_scale,
    xr_matrix4x4f_identity, xr_matrix4x4f_invert_rigid_body, xr_matrix4x4f_multiply,
    xr_matrix4x4f_transform_vector3f, GraphicsAPI, XrFovf, XrMatrix4x4f, XrQuaternionf, XrVector3f,
};
use openxr_sys::Time;
use std::error::Error;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::mem::size_of;
use std::time::{SystemTime, UNIX_EPOCH};

//

pub struct Renderer<'a> {
    pub program: FlatColorShader,
    pub buffers: VertexBufferBundle<'a, GLfloat, u8>,
    pub indices_len: usize,
    pub suzanne: Suzanne,
    pub text_message: TextMessage,
}

impl<'a> Renderer<'a> {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, Box<dyn Error>> {
        let program = FlatColorShader::new()?;

        program.program.use_()?;

        let mut buffers = VertexBufferBundle::<'static, GLfloat, u8>::new()?;
        let indices_len = {
            let bindings = buffers.bind_mut(gpu_state)?;
            Self::configure_vertex_attributes(&bindings, &program.program, 2)?;

            const COLOR_TRIANGLE: [GLfloat; 3 * 5] = [
                -0.5, -0.5, 0.0, 1.0, 0.0, //
                0.0, 0.5, 0.0, 0.0, 1.0, //
                0.5, -0.5, 1.0, 0.0, 0.0,
            ];
            bindings.vertex_buffer.load(&COLOR_TRIANGLE)?;

            static INDICES: [u8; 3] = [0, 1, 2];
            let indices = &INDICES;
            bindings.index_buffer.load(indices)?;
            indices.len()
        };

        let rval = Renderer {
            buffers,
            indices_len,
            program,
            suzanne: Suzanne::new(gpu_state)?,
            text_message: TextMessage::new(gpu_state)?,
        };

        Ok(rval)
    }

    pub fn draw(
        &self,
        fov: &XrFovf,
        rotation: &XrQuaternionf,
        translation: &XrVector3f,
        _time: Time,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        // building the uniforms

        let (theta, rotation_matrix) = rotation_matrix_for_now();

        //

        unsafe {
            let green = (theta.sin() + 1.0) * 0.5;
            gl::ClearColor(0.0, green, 0.3, 1.0)
        };
        explode_if_gl_error()?;
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT) };
        explode_if_gl_error()?;

        unsafe { gl::Enable(gl::DEPTH_TEST) };
        explode_if_gl_error()?;

        //

        let matrix = {
            let projection_matrix = xr_matrix4x4f_create_projection_fov(
                GraphicsAPI::GraphicsOpenGL,
                fov,
                0.01,
                10_000.0,
            );
            //log::debug!("matrix = {}", debug_string_matrix(&projection_matrix),);
            let view_matrix = xr_matrix4x4f_create_translation_rotation_scale(
                translation,
                rotation,
                &XrVector3f::default_scale(),
            );
            let inverse_view_matrix = xr_matrix4x4f_invert_rigid_body(&view_matrix);

            //log::debug!("rotation {:?}", rotation);
            // log::debug!("translation {:?}", translation);

            if false {
                let tmp = xr_matrix4x4f_transform_vector3f(
                    &inverse_view_matrix,
                    &XrVector3f {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                );
                let xyz = xr_matrix4x4f_transform_vector3f(&projection_matrix, &tmp);
                log::debug!("transformed {:?}", xyz);
            }

            xr_matrix4x4f_multiply(
                //
                &projection_matrix,   //
                &inverse_view_matrix, //
            )
        };

        {
            let model = xr_matrix4x4f_create_translation(1.0, 0.0, -2.0);
            let model = xr_matrix4x4f_multiply(&model, &rotation_matrix);
            self.paint_color_triangle(&matrix, &model, gpu_state)?;
        }

        {
            let model = {
                let upright = matrix_rotation_about_x(-FRAC_PI_2);
                let translate = xr_matrix4x4f_create_translation(-1.0, -0.5, -2.0);
                let scale = xr_matrix4x4f_create_scale(0.5, 0.5, 0.5);
                let model = scale;
                let model = xr_matrix4x4f_multiply(&upright, &model);
                let model = xr_matrix4x4f_multiply(&rotation_matrix, &model);
                xr_matrix4x4f_multiply(&translate, &model)
            };
            let identity = xr_matrix4x4f_identity();
            self.suzanne.draw(
                &matrix,
                &identity,
                &model,
                &[0.0, 1.0, 0.0],
                &[0.0, 0.0, 1.0],
                self.suzanne.index_count(),
                gpu_state,
            )?;
        }

        {
            let model = {
                let translate = xr_matrix4x4f_create_translation(0.0, -0.5, -1.0);
                let s = 0.2;
                let scale = xr_matrix4x4f_create_scale(s, s, s);
                let model = scale;
                // let model = xr_matrix4x4f_multiply(&upright, &model);
                let model = xr_matrix4x4f_multiply(&rotation_matrix, &model);
                xr_matrix4x4f_multiply(&translate, &model)
            };
            let identity = xr_matrix4x4f_identity();
            self.text_message.draw(
                &matrix,
                &identity,
                &model,
                self.text_message.index_count(),
                gpu_state,
            )
        }
    }

    fn paint_color_triangle(
        &self,
        pv_matrix: &[f32; 16],
        model: &XrMatrix4x4f,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        let program = &self.program.program;
        program.use_().unwrap();

        let matrix = xr_matrix4x4f_multiply(pv_matrix, model);

        self.program.set_params(&matrix);

        if let Ok(location) = program.get_uniform_location("matrix") {
            //log::debug!("matrix location {}", location);
            program.set_mat4u(location as GLint, &matrix).unwrap();
        }

        let binding = self.buffers.bind(gpu_state)?;

        binding.draw_elements(gl::TRIANGLES, self.indices_len as i32, 0)?;

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

    fn configure_vertex_attributes<AT: GLBufferType, IT>(
        buffers: &BoundBuffersMut<AT, IT>,
        program: &Program,
        xyz_width: i32,
    ) -> Result<(), GLErrorWrapper> {
        let stride = xyz_width + 3;
        buffers.rig_one_attribute_by_name(program, "position", xyz_width, stride, 0)?;
        buffers.rig_one_attribute_by_name(program, "color", 3, stride, xyz_width)?;
        /*        Self::rig_one_va(program, "position", xyz_width, stride, 0)?;
                Self::rig_one_va(program, "color", 3, stride, xyz_width)?;
        */
        Ok(())
    }
}

fn rotation_matrix_for_now() -> (f32, [f32; 16]) {
    let theta = if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let tm = duration.as_millis();
        let phase = tm % 5000;
        TAU * phase as f32 / 5000.0
    } else {
        0.0
    };
    let rotation_matrix = if true {
        matrix_rotation_about_y(theta)
    } else {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0f32,
        ]
    };
    (theta, rotation_matrix)
}

#[rustfmt::skip]
pub fn matrix_rotation_about_z(theta: f32) -> [f32; 16] {
    [
        theta.cos(), theta.sin(), 0.0, 0.0, //
        -theta.sin(), theta.cos(), 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0f32,
    ]
}

#[rustfmt::skip]
pub fn matrix_rotation_about_y(theta: f32) -> [f32; 16] {
    [
        theta.cos(), 0.0, theta.sin(), 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        -theta.sin(), 0.0, theta.cos(), 0.0, //
        0.0, 0.0, 0.0, 1.0f32,
    ]
}

#[rustfmt::skip]
pub fn matrix_rotation_about_x(theta: f32) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, theta.cos(), theta.sin(), 0.0,
        0.0, -theta.sin(), theta.cos(), 0.0,
        0.0, 0.0, 0.0, 1.0f32,
    ]
}

//

pub struct Suzanne {
    phong: SunPhongShader,
    buffers: VertexBufferBundle<'static, GLfloat, GLushort>,
    index_count: GLsizei,
}

impl Suzanne {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        let mut buffers = VertexBufferBundle::new()?;
        let binding = buffers.bind_mut(gpu_state)?;

        let xyzabc = &crate::suzanne::XYZABC;
        binding.vertex_buffer.load(xyzabc)?;

        let indices = &crate::suzanne::TRIANGLE_INDICES;
        binding.index_buffer.load(indices)?;

        let phong = SunPhongShader::new()?;
        if true {
            // buffers.bind_primitive()?; // is this redundant ? XXX
            phong.rig_attribute_arrays(&binding.plain())?;
        }

        drop(binding);

        Ok(Self {
            phong,
            buffers,
            index_count: indices.len() as GLsizei,
        })
    }

    pub fn index_count(&self) -> GLsizei {
        self.index_count
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        projection: &XrMatrix4x4f,
        view: &[f32; 16],
        model: &[f32; 16],
        sun_direction: &[f32; 3],
        color: &[f32; 3],
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.phong.draw(
            projection,
            view,
            model,
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
    program: RawTextureShader,
    buffers: VertexBufferBundle<'static, GLfloat, GLushort>,
    index_count: GLsizei,
    texture: Texture,
}

impl TextMessage {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        let mut buffers = VertexBufferBundle::new().unwrap();
        let binding = buffers.bind_mut(gpu_state).unwrap();

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
        binding.vertex_buffer.load_owned(xyuv).unwrap();

        let indices = &[0, 1, 2, 3];
        binding.index_buffer.load(indices).unwrap();

        let program = RawTextureShader::new().unwrap();

        program.program.use_().unwrap();
        binding
            .rig_one_attribute(program.sal_position, 3, 5, 0)
            .unwrap();
        binding
            .rig_one_attribute(program.sal_tex_coord, 2, 5, 3)
            .unwrap();

        drop(binding);

        let rval = Self {
            program,
            buffers,
            index_count: indices.len() as GLsizei,
            texture: text_painting::banana(tex_width, tex_height).unwrap(),
        };
        Ok(rval)
    }

    pub fn index_count(&self) -> GLsizei {
        self.index_count
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        projection: &XrMatrix4x4f,
        view: &[f32; 16],
        model: &[f32; 16],
        n_indices: GLsizei,
        gpu_state: &mut GPUState,
    ) -> Result<(), GLErrorWrapper> {
        self.program.draw(
            projection,
            view,
            model,
            &self.texture,
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
