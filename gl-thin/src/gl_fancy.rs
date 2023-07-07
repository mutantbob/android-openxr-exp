use crate::gl_helper;
use crate::gl_helper::{
    explode_if_gl_error, gl_offset_for, ArrayBufferType, Buffer, ElementArrayBufferType,
    GLBufferType, GLErrorWrapper, Program, VertexArray,
};
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use std::mem::size_of;

pub struct GPUState {}

impl GPUState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn bind_vertex_array_and_buffers<'a, AT, IT>(
        &'a mut self,
        vertex_array: &'a VertexArray,
        vertex_buffer: &'a Buffer<ArrayBufferType, AT>,
        index_buffer: &'a Buffer<ElementArrayBufferType, IT>,
    ) -> Result<BoundBuffers<'a, AT, IT>, GLErrorWrapper> {
        vertex_array.bind()?;
        vertex_buffer.bind()?;
        index_buffer.bind()?;
        Ok(BoundBuffers::new(
            self,
            vertex_array,
            vertex_buffer,
            index_buffer,
        ))
    }

    pub fn bind_vertex_array_and_buffers_mut<'a, 'g, 'd, AT, IT>(
        &'g mut self,
        vertex_array: &'a VertexArray,
        vertex_buffer: &'a mut Buffer<'d, ArrayBufferType, AT>,
        index_buffer: &'a mut Buffer<'d, ElementArrayBufferType, IT>,
    ) -> Result<BoundBuffersMut<'a, 'g, 'd, AT, IT>, GLErrorWrapper> {
        vertex_array.bind()?;
        vertex_buffer.bind()?;
        index_buffer.bind()?;

        Ok(BoundBuffersMut::new(
            self,
            vertex_array,
            vertex_buffer,
            index_buffer,
        ))
    }
}

//

pub struct BoundBuffers<'a, AT, IT> {
    pub gpu_state: &'a GPUState,
    pub vertex_array: &'a VertexArray,
    pub vertex_buffer: &'a Buffer<'a, ArrayBufferType, AT>,
    pub index_buffer: &'a Buffer<'a, ElementArrayBufferType, IT>,
}

impl<'a, AT, IT> BoundBuffers<'a, AT, IT> {
    fn new(
        gpu_state: &'a GPUState,
        vertex_array: &'a VertexArray,
        vertex_buffer: &'a Buffer<'a, ArrayBufferType, AT>,
        index_buffer: &'a Buffer<'a, ElementArrayBufferType, IT>,
    ) -> Self {
        Self {
            gpu_state,
            vertex_array,
            vertex_buffer,
            index_buffer,
        }
    }

    /// # Arguments
    /// * `program_attribute_location` -  is the result of calling gl::GetAttribLocation for the program you will be using
    /// * `attribute_array_width` - would be 3 for a vec3 or 2 for a vec2
    /// * `stride` - is how many floats are in a row, because often data is packed with multiple attributes per row.  For example, XYZUV data would have stride 5 and probably two attributes with width 3 (for xyz) and 2 (for uv)
    /// * `offset` - how many floats are between the beginning of the "row" and this attribute's data.  The UV data in an XYZUV data set would have offset 3 since the UV appears after the XYZ in each row.
    pub fn rig_one_attribute<T: GLBufferType>(
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
                T::TYPE_CODE,
                gl::FALSE,
                stride * size_of::<T>() as GLsizei,
                gl_helper::gl_offset_for::<T>(offset),
            );
        }
        explode_if_gl_error()
    }

    pub fn rig_one_attribute_by_name<T: GLBufferType>(
        &self,
        program: &Program,
        name: &str,
        attribute_array_width: GLint,
        stride: GLsizei,
        offset: GLsizei,
    ) -> Result<(), GLErrorWrapper> {
        let loc = program.get_attribute_location(name)?;
        self.rig_one_attribute::<T>(loc, attribute_array_width, stride, offset)?;

        unsafe { gl::EnableVertexAttribArray(loc) };
        explode_if_gl_error()
    }
}

impl<'a, AT, IT: GLBufferType> BoundBuffers<'a, AT, IT> {
    pub fn draw_elements(
        &self,
        mode: GLenum,
        n_indices: GLsizei,
        offset: GLsizei,
    ) -> Result<(), GLErrorWrapper> {
        let offset = unsafe { gl_offset_for::<IT>(offset) };
        unsafe {
            gl::DrawElements(mode, n_indices, IT::TYPE_CODE, offset);
        }
        explode_if_gl_error()
    }
}

impl<'a, AT, IT> Drop for BoundBuffers<'a, AT, IT> {
    fn drop(&mut self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }
}
//

pub struct BoundBuffersMut<'a, 'g, 'd, AT, IT> {
    pub gpu_state: &'g GPUState,
    pub vertex_array: &'a VertexArray,
    pub vertex_buffer: &'a mut Buffer<'d, ArrayBufferType, AT>,
    pub index_buffer: &'a mut Buffer<'d, ElementArrayBufferType, IT>,
}

impl<'a, 'g, 'd, AT, IT> BoundBuffersMut<'a, 'g, 'd, AT, IT> {
    // XXX I am worried that this might not mean what I think it means.
    pub fn plain(&self) -> BoundBuffers<AT, IT> {
        BoundBuffers {
            gpu_state: self.gpu_state,
            vertex_array: self.vertex_array,
            vertex_buffer: self.vertex_buffer,
            index_buffer: self.index_buffer,
        }
    }
}

impl<'a, 'g, 'd, AT, IT> BoundBuffersMut<'a, 'g, 'd, AT, IT> {
    fn new(
        gpu_state: &'g GPUState,
        vertex_array: &'a VertexArray,
        vertex_buffer: &'a mut Buffer<'d, ArrayBufferType, AT>,
        index_buffer: &'a mut Buffer<'d, ElementArrayBufferType, IT>,
    ) -> Self {
        Self {
            gpu_state,
            vertex_array,
            vertex_buffer,
            index_buffer,
        }
    }
}

impl<'a, 'g, 'd, AT: GLBufferType, IT> BoundBuffersMut<'a, 'g, 'd, AT, IT> {
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
                AT::TYPE_CODE,
                gl::FALSE,
                stride * size_of::<AT>() as GLsizei,
                gl_helper::gl_offset_for::<AT>(offset),
            );
        }
        explode_if_gl_error()?;

        unsafe { gl::EnableVertexAttribArray(program_attribute_location) };
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
        self.rig_one_attribute(loc, attribute_array_width, stride, offset)
    }
}

impl<'a, 'g, 'd, AT, IT> Drop for BoundBuffersMut<'a, 'g, 'd, AT, IT> {
    fn drop(&mut self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }
}
//

pub struct VertexBufferBundle<'a, AT, IT> {
    pub vertex_array: VertexArray,
    pub vertex_buffer: Buffer<'a, ArrayBufferType, AT>,
    pub index_buffer: Buffer<'a, ElementArrayBufferType, IT>,
}

impl<'a, AT, IT> VertexBufferBundle<'a, AT, IT> {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        Ok(Self {
            vertex_array: VertexArray::new()?,
            vertex_buffer: Buffer::new()?,
            index_buffer: Buffer::new()?,
        })
    }

    pub fn bind(
        &'a self,
        gpu_state: &'a mut GPUState,
    ) -> Result<BoundBuffers<'a, AT, IT>, GLErrorWrapper> {
        self.vertex_array.bind()?;
        self.vertex_buffer.bind()?;
        self.index_buffer.bind()?;
        gpu_state.bind_vertex_array_and_buffers(
            &self.vertex_array,
            &self.vertex_buffer,
            &self.index_buffer,
        )
    }

    pub fn bind_mut<'s, 'g>(
        &'s mut self,
        gpu_state: &'g mut GPUState,
    ) -> Result<BoundBuffersMut<'s, 'g, 'a, AT, IT>, GLErrorWrapper> {
        gpu_state.bind_vertex_array_and_buffers_mut(
            &self.vertex_array,
            &mut self.vertex_buffer,
            &mut self.index_buffer,
        )
    }

    pub fn bind_primitive(&self) -> Result<(), GLErrorWrapper> {
        self.vertex_array.bind()?;
        self.vertex_buffer.bind()?;
        self.index_buffer.bind()
    }
}
