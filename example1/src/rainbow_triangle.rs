use crate::gl_helper;
use crate::gl_helper::{
    explode_if_gl_error, ArrayBufferType, Buffer, ElementArrayBufferType, GLErrorWrapper, Program,
    VertexArray,
};
use crate::linear::{
    xr_matrix4x4f_create_projection_fov, xr_matrix4x4f_create_translation,
    xr_matrix4x4f_create_translation_rotation_scale, xr_matrix4x4f_invert_rigid_body,
    xr_matrix4x4f_multiply, xr_matrix4x4f_transform_vector3f, GraphicsAPI, XrFovf, XrMatrix4x4f,
    XrQuaternionf, XrVector3f,
};
use crate::sun_phong_shader::{GeometryBuffer, SunPhongShader};
use gl::types::{GLfloat, GLint, GLsizei, GLushort};
use openxr_sys::Time;
use std::error::Error;
use std::f32::consts::TAU;
use std::mem::size_of;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Renderer<'a> {
    pub vertex_array: VertexArray,
    pub vertex_buffer: Buffer<'a, ArrayBufferType, GLfloat>,
    pub index_buffer: Option<Buffer<'a, ElementArrayBufferType, GLushort>>,
    pub indices_len: usize,
    pub program: Program,
    //pub phong: SunPhongShader,
    pub suzanne: Suzanne,
}

impl<'a> Renderer<'a> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let program = gl_helper::Program::compile(
            "

                uniform mat4 matrix;

                attribute vec3 position;
                attribute vec3 color;

                varying vec3 vColor;

                void main() {
                    gl_Position = matrix * vec4(position*0.5, 1.0) ;
                    vColor = color;
                }
            ",
            "
                varying vec3 vColor;

                void main() {
                    gl_FragColor = vec4(vColor, 1.0);
                }
            ",
        )
        .unwrap();

        program.use_()?;

        let vertex_array = VertexArray::new()?;
        vertex_array.bind()?;
        // building the vertex buffer, which contains all the vertices that we will draw
        let mut vertex_buffer = gl_helper::Buffer::new()?;

        const COLOR_TRIANGLE: [GLfloat; 3 * 5] = [
            -0.5, -0.5, 0.0, 1.0, 0.0, //
            0.0, 0.5, 0.0, 0.0, 1.0, //
            0.5, -0.5, 1.0, 0.0, 0.0,
        ];
        vertex_buffer.load(&crate::suzanne::XYZABC)?;

        let (index_buffer, indices_len) = if true {
            let mut index_buffer = gl_helper::Buffer::new()?;
            static INDICES: [u16; 3] = [0u16, 1, 2];
            let indices = &crate::suzanne::TRIANGLE_INDICES;
            index_buffer.load(indices)?;
            (Some(index_buffer), indices.len())
        } else {
            (None, 3)
        };

        Self::configure_vertex_attributes(&program)?;

        let rval = Renderer {
            vertex_array,
            vertex_buffer,
            index_buffer,
            indices_len,
            program,
            suzanne: Suzanne::new()?,
        };

        Ok(rval)
    }

    pub fn draw(
        &self,
        fov: &XrFovf,
        rotation: &XrQuaternionf,
        translation: &XrVector3f,
        _time: Time,
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

            let pv = xr_matrix4x4f_multiply(
                //
                &projection_matrix,   //
                &inverse_view_matrix, //
            );

            let model = xr_matrix4x4f_create_translation(0.0, 0.0, -0.5);
            let model = xr_matrix4x4f_multiply(&model, &rotation_matrix);
            xr_matrix4x4f_multiply(&pv, &model)
        };

        if true {
            self.paint_color_triangle(&matrix)
        } else {
            //self.suzanne.vertex_array.bind()?;
            // unsafe { gl::BindVertexArray(0) };
            self.suzanne.phong.draw(
                &matrix,
                &XrMatrix4x4f::default(),
                &XrMatrix4x4f::default(),
                &[0.0, 1.0, 0.0],
                &[0.0, 0.0, 1.0],
                &self.suzanne,
                self.suzanne.index_count(),
            )
        }
    }

    fn paint_color_triangle(&self, matrix: &[f32; 16]) -> Result<(), GLErrorWrapper> {
        self.program.use_().unwrap();

        if let Ok(location) = self.program.get_uniform_location("matrix") {
            //log::debug!("matrix location {}", location);
            self.program.set_mat4u(location as GLint, matrix).unwrap();
        }

        self.vertex_array.bind()?;

        self.vertex_buffer.bind()?;
        if let Some(index_buffer) = self.index_buffer.as_ref() {
            index_buffer.bind()?;
        }

        unsafe {
            //Self::configure_vertex_attributes(&self.program);

            gl::DrawElements(
                gl::TRIANGLES,
                self.indices_len as i32,
                gl::UNSIGNED_SHORT,
                gl_helper::gl_offset_for::<GLushort>(0),
            );
            explode_if_gl_error()?;
            // too lazy to unbind
        }
        Ok(())
    }

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

    fn configure_vertex_attributes(program: &Program) -> Result<(), GLErrorWrapper> {
        Self::rig_one_va(program, "position", 3, 6, 0)?;
        Self::rig_one_va(program, "color", 3, 6, 3)?;
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

//

pub struct Suzanne {
    phong: SunPhongShader,
    vertex_array: VertexArray,
    vertex_buffer: Buffer<'static, ArrayBufferType, f32>,
    index_buffer: Buffer<'static, ElementArrayBufferType, GLushort>,
    index_count: GLsizei,
}

impl Suzanne {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let vertex_array = VertexArray::new()?;
        vertex_array.bind()?;

        let mut vertex_buffer = Buffer::new()?;
        let xyzabc = &crate::suzanne::XYZABC;
        /*let xyzabc = &[
            -0.5, 0.5, 0.0, 0.0, 0.0, 1.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0, 0.0, -0.5, 0.0, 0.0, 0.0,
            1.0,
        ];*/
        vertex_buffer.load(xyzabc)?;

        let mut index_buffer = Buffer::new()?;
        let indices = &crate::suzanne::TRIANGLE_INDICES;
        // let indices = &[0, 1, 2];
        index_buffer.load(indices)?;

        let phong = SunPhongShader::new()?;
        phong.rig_attribute_arrays();

        Ok(Self {
            phong,
            vertex_array,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as GLsizei,
        })
    }

    pub fn index_count(&self) -> GLsizei {
        self.index_count
    }
}

impl GeometryBuffer for Suzanne {
    fn activate(&self) {
        self.vertex_array.bind().unwrap();
        self.vertex_buffer.bind().unwrap();
        self.index_buffer.bind().unwrap();
    }

    fn deactivate(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
    }
}
