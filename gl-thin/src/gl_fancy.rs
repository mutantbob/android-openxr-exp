use crate::gl_helper;
use crate::gl_helper::{
    bytes_per_pixel, explode_if_gl_error, gl_offset_for, ArrayBufferType, Buffer, BufferOwnership,
    BufferTarget, ElementArrayBufferType, GLBufferType, GLErrorWrapper, Program, Texture,
    VertexArray,
};
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use std::mem::size_of;

/// The OpenGL API has quite a bit of state.
/// I have barely scratched the surface of encoding it in Rust's type system,
/// and I'm not confident that I am accurately representing the characteristics.
/// * vertexarray bindings (maybe done?)
/// * active texture slot bindings (not even started)
/// * what else?
pub struct GPUState {
    active_texture_unit: ActiveTextureUnit,
}

impl GPUState {
    pub fn new() -> Self {
        Self {
            active_texture_unit: ActiveTextureUnit(0),
        }
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
        vertex_buffer.bind()?; // XXX rework this
        index_buffer.bind()?;

        Ok(BoundBuffersMut::new(
            self,
            vertex_array,
            vertex_buffer,
            index_buffer,
        ))
    }

    pub fn set_active_texture(&mut self, idx: ActiveTextureUnit) -> Result<(), GLErrorWrapper> {
        self.active_texture_unit = idx;
        unsafe { gl::ActiveTexture(self.active_texture_unit.gl_arg()) };
        explode_if_gl_error()
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
    pub vertex_buffer: OneBoundBuffer<'a, 'g, 'd, ArrayBufferType, AT>,
    pub index_buffer: OneBoundBuffer<'a, 'g, 'd, ElementArrayBufferType, IT>,
}

impl<'a, 'g, 'd, AT, IT> BoundBuffersMut<'a, 'g, 'd, AT, IT> {
    // XXX I am worried that this might not mean what I think it means.
    pub fn plain(&self) -> BoundBuffers<AT, IT> {
        BoundBuffers {
            gpu_state: self.gpu_state,
            vertex_array: self.vertex_array,
            vertex_buffer: self.vertex_buffer.buffer,
            index_buffer: self.index_buffer.buffer,
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
        let vertex_buffer = OneBoundBuffer {
            gpu_state,
            buffer: vertex_buffer,
        };
        let index_buffer = OneBoundBuffer {
            gpu_state,
            buffer: index_buffer,
        };
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

    /// # params
    /// `stride` - the offset between the beginning of one sample and the beginning of the subsequent sample
    ///
    /// `attributes` - an iterable of (attribute_location, attribute_width, offset) tuples.
    ///
    /// # example
    /// If you have some packed XYZUV values this will have a stride of 5, the XYZ at offset 0,  and UV at offset 3 (immediately after the XYZ)
    /// ```
    /// # use gl::types::GLuint;
    /// # use gl_thin::gl_fancy::VertexBufferBundle;
    /// # struct Program { sal_xyz: GLuint,
    /// # sal_uv: GLuint,}
    /// # fn x<AT,IT>(buffer: &VertexBufferBundle<AT,IT>, program: &Program) {}
    /// buffer.rig_multi_attributes(5, &[
    ///     (program.sal_xyz, 3, 0),
    ///     (program.sal_uv, 2, 3),
    ///   ]);
    /// # }
    /// ```
    pub fn rig_multi_attributes<'i>(
        &self,
        stride: GLsizei,
        attributes: impl IntoIterator<Item = &'i (GLuint, GLint, GLsizei)>,
    ) -> Result<(), GLErrorWrapper> {
        for (location, attribute_width, offset) in attributes {
            self.rig_one_attribute(*location, *attribute_width, stride, *offset)?;
        }
        Ok(())
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

pub struct OneBoundBuffer<'a, 'g, 'd, B, T> {
    pub gpu_state: &'g GPUState,
    pub buffer: &'a mut Buffer<'d, B, T>,
}

impl<'a, 'g, 'd, B: BufferTarget, T> OneBoundBuffer<'a, 'g, 'd, B, T> {
    pub fn load(&mut self, values: &'d [T]) -> Result<(), GLErrorWrapper> {
        self.buffer.load(values)
    }

    pub fn load_any(&mut self, values: BufferOwnership<'d, T>) -> Result<(), GLErrorWrapper> {
        unsafe { self.buffer.load_any(values) }
    }

    pub fn load_owned(&mut self, values: Vec<T>) -> Result<(), GLErrorWrapper> {
        self.buffer.load_owned(values)
    }
}
//

pub struct VertexBufferBundle<'a, AT, IT> {
    pub vertex_array: VertexArray,
    pub vertex_buffer: Buffer<'a, ArrayBufferType, AT>,
    pub index_buffer: Buffer<'a, ElementArrayBufferType, IT>,
    pub index_count: usize,
}

impl<'a, AT, IT> VertexBufferBundle<'a, AT, IT> {
    pub fn incomplete() -> Result<Self, GLErrorWrapper> {
        Ok(Self {
            vertex_array: VertexArray::new()?,
            vertex_buffer: Buffer::new()?,
            index_buffer: Buffer::new()?,
            index_count: 0,
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
        self.vertex_array.bind()?;
        self.vertex_buffer.bind()?;
        self.index_buffer.bind()?;
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

impl<'a, AT: GLBufferType, IT: GLBufferType> VertexBufferBundle<'a, AT, IT> {
    /// Creates a VertexBufferBundle,
    /// binds some data to the buffers,
    /// and binds the vertex attributes.
    /// That last step trips up some people.
    ///
    /// # parameters
    /// `attributes` - an iterable of (attribute_location, attribute_width, offset) tuples.  See [BoundBuffersMut::rig_multi_attributes] for more details
    ///
    /// # Example
    /// ```
    /// # use gl::types::GLsizei;
    /// # use gl_thin::gl_fancy::{GPUState, VertexBufferBundle};
    /// # use gl_thin::gl_helper::GLErrorWrapper;
    /// # fn x(gpu_state: &mut GPUState, xyzuv:&[f32], indices:&[u16], program: &FlatColorShader) {
    /// let vbb = VertexBufferBundle::new(
    ///                 gpu_state,
    ///                 xyzuv.into(),
    ///                 indices.into(),
    ///                 3+2,
    ///                 &[(program.sal_position, 3, 0), (program.sal_tex_coord, 2, 3)],
    ///             )?;
    /// # }
    /// // later
    /// # fn draw(vbb :&VertexBufferBundle<f32,u16>, n_indices: GLsizei, program: &FlatColorShader, gpu_state: &mut GPUState) {
    ///     program.use_()?;
    ///     let bound = vbb.bind(gpu_state)?;
    ///     bound.draw_elements(gl::TRIANGLES, n_indices, 0)?;
    /// # }
    ///```
    pub fn new<'i>(
        gpu_state: &mut GPUState,
        vertex_data: BufferOwnership<'a, AT>,
        index_data: BufferOwnership<'a, IT>,
        vertex_data_stride: GLsizei,
        attributes: impl IntoIterator<Item = &'i (GLuint, GLint, GLsizei)>,
    ) -> Result<Self, GLErrorWrapper> {
        let mut rval = Self::incomplete()?;
        let index_count = index_data.as_slice().len();
        {
            let mut bound = rval.bind_mut(gpu_state)?;
            bound.vertex_buffer.load_any(vertex_data)?;
            bound.index_buffer.load_any(index_data)?;
            bound.rig_multi_attributes(vertex_data_stride, attributes)?;
        }
        rval.index_count = index_count;
        Ok(rval)
    }
}

//

pub struct ActiveTextureUnit(pub u32);

impl ActiveTextureUnit {
    pub fn gl_arg(&self) -> GLenum {
        gl::TEXTURE0 + self.0
    }
}

//

pub struct BoundTexture<'g, 't> {
    // prevent anyone else from modifying the active texture unit until we are done using this object
    #[allow(dead_code)]
    lock: &'g ActiveTextureUnit,
    // probably gl::TEXTURE_2D
    target: GLenum,
    tex: &'t Texture,
}

impl<'g, 't> BoundTexture<'g, 't> {
    pub fn new(
        gpu_state: &'g GPUState,
        arg: &'t Texture,
        target: GLenum,
    ) -> Result<Self, GLErrorWrapper> {
        arg.bind(target)?;
        Ok(Self {
            lock: &gpu_state.active_texture_unit,
            target,
            tex: arg,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure<T: GLBufferType>(
        &self,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: GLenum,
    ) -> Result<(), GLErrorWrapper> {
        unsafe {
            gl::TexImage2D(
                self.target,
                level,
                internal_format,
                width,
                height,
                border,
                format,
                T::TYPE_CODE,
                std::ptr::null(),
            )
        };
        explode_if_gl_error()
    }

    pub fn attach(
        &self,
        target: GLenum,
        attachment: GLenum,
        level: i32,
    ) -> Result<(), GLErrorWrapper> {
        let texture = *self.tex.0.unwrap();
        unsafe { gl::FramebufferTexture2D(target, attachment, self.target, texture, level) };
        explode_if_gl_error()
    }

    pub fn get_width(&self) -> Result<GLint, GLErrorWrapper> {
        let mut rval = 0;
        unsafe { gl::GetTexLevelParameteriv(self.target, 0, gl::TEXTURE_WIDTH, &mut rval) };
        explode_if_gl_error()?;

        Ok(rval)
    }

    pub fn get_height(&self) -> Result<GLint, GLErrorWrapper> {
        let mut rval = 0;
        unsafe { gl::GetTexLevelParameteriv(self.target, 0, gl::TEXTURE_HEIGHT, &mut rval) };
        explode_if_gl_error()?;

        Ok(rval)
    }

    pub fn get_dimensions(&self) -> Result<(GLint, GLint), GLErrorWrapper> {
        let mut width = 0;
        unsafe { gl::GetTexLevelParameteriv(self.target, 0, gl::TEXTURE_WIDTH, &mut width) };
        explode_if_gl_error()?;

        let mut height = 0;
        unsafe { gl::GetTexLevelParameteriv(self.target, 0, gl::TEXTURE_HEIGHT, &mut height) };
        explode_if_gl_error()?;

        Ok((width, height))
    }

    pub fn write_pixels_and_generate_mipmap<T: GLBufferType>(
        &mut self,
        level: GLint,
        internal_format: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        pixels: &[T],
    ) -> Result<(), GLErrorWrapper> {
        self.write_pixels(level, internal_format, width, height, format, pixels)?;
        self.generate_mipmap()
    }

    /// Remember to populate the mipmap by either writing all the different mipmap `level`s or using `self.generate_mipmap()`
    pub fn write_pixels<T: GLBufferType>(
        &mut self,
        level: GLint,
        internal_format: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        pixels: &[T],
    ) -> Result<(), GLErrorWrapper> {
        let bpp = bytes_per_pixel::<T>(format)?;
        if (width * height) as usize * bpp != pixels.len() {
            return Err(GLErrorWrapper::with_message2(format!(
                "size mismatch : {}*{}*{} != {}",
                width,
                height,
                bpp,
                pixels.len()
            )));
        }

        unsafe {
            gl::TexImage2D(
                self.target,
                level,
                internal_format,
                width,
                height,
                0,
                format,
                T::TYPE_CODE,
                pixels.as_ptr() as *const _,
            );
        }
        explode_if_gl_error()
    }

    pub fn generate_mipmap(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::GenerateMipmap(self.target) };
        explode_if_gl_error()
    }
}
