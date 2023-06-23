use crate::gl_helper;
use crate::gl_helper::{
    explode_if_gl_error, ArrayBufferType, Buffer, ElementArrayBufferType, GLErrorWrapper, Program,
    VertexArray,
};
use gl::types::{GLfloat, GLint, GLsizei, GLuint, GLushort};
use std::mem::size_of;

pub struct VertexBufferBundle<'a> {
    pub vertex_array: VertexArray,
    pub vertex_buffer: Buffer<'a, ArrayBufferType, GLfloat>,
    pub index_buffer: Buffer<'a, ElementArrayBufferType, GLushort>,
}

impl<'a> VertexBufferBundle<'a> {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        Ok(Self {
            vertex_array: VertexArray::new()?,
            vertex_buffer: Buffer::new()?,
            index_buffer: Buffer::new()?,
        })
    }

    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        self.vertex_array.bind()?;
        self.vertex_buffer.bind()?;
        self.index_buffer.bind()
    }

    /// # Arguments
    /// * `program_attribute_location` -  is the result of calling gl::GetAttribLocation for the program you will be using
    /// * `attribute_array_width` - would be 3 for a vec3 or 2 for a vec2
    /// * `stride` - is how many floats are in a row, because often data is packed with multiple attributes per row.  For example, XYZUV data would have stride 5 and probably two attributes with width 3 (for xyz) and 2 (for uv)
    /// * `offset` - how many floats are between the beginning of the "row" and this attribute's data.  The UV data in an XYZUV data set would have offset 3 since the UV appears after the XYZ in each row.
    pub fn rig_one_attribute(
        &self,
        program_attribute_location: GLuint,
        attribute_array_width: GLint,
        stride: GLsizei,
        offset: GLsizei,
    ) -> Result<(), GLErrorWrapper> {
        unsafe {
            gl::VertexAttribPointer(
                program_attribute_location,
                attribute_array_width,
                gl::FLOAT,
                gl::FALSE,
                stride * size_of::<GLfloat>() as GLsizei,
                gl_helper::gl_offset_for::<GLfloat>(offset),
            );
        }
        explode_if_gl_error()
    }

    pub fn rig_one_attribute_by_name(
        &self,
        program: &Program,
        name: &str,
        attribute_array_width: GLint,
        stride: GLsizei,
        offset: GLsizei,
    ) -> Result<(), GLErrorWrapper> {
        let loc = program.get_attribute_location(name)?;
        self.rig_one_attribute(loc, attribute_array_width, stride, offset);

        unsafe { gl::EnableVertexAttribArray(loc) };
        explode_if_gl_error()
    }
}
