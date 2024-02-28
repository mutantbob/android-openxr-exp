use crate::gl_helper;
use crate::gl_helper::{
    bytes_per_pixel, explode_if_gl_error, gl_offset_for, ArrayBufferType, Buffer, BufferOwnership,
    BufferTarget, ElementArrayBufferType, GLBufferType, GLErrorWrapper, Program, Texture,
    VertexArray,
};
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use std::marker::PhantomData;
use std::mem::size_of;
use std::rc::Rc;

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

pub struct OneBoundBuffer<'a, 'g, 'd, B, T> {
    #[allow(dead_code)]
    gpu_state: &'g GPUState,
    buffer: &'a mut Buffer<'d, B, T>,
}

impl<'a, 'g, 'd, B: BufferTarget, T> OneBoundBuffer<'a, 'g, 'd, B, T> {
    pub fn new(
        gpu_state: &'g GPUState,
        buffer: &'a mut Buffer<'d, B, T>,
    ) -> Result<Self, GLErrorWrapper> {
        buffer.bind()?;
        Ok(Self::new_after_bind(gpu_state, buffer))
    }

    pub(crate) fn new_after_bind(
        gpu_state: &'g GPUState,
        buffer: &'a mut Buffer<'d, B, T>,
    ) -> Self {
        Self { gpu_state, buffer }
    }

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

/// Used to store a set of vertex and index buffers.
/// Is often used to make a [VertexBufferBundle] by adding a [VertexArray] mapping object.
///
/// # See also
/// [VertexBufferBundle::from_buffers()]
pub struct VertexBufferLite<'a, AT, IT> {
    pub vertex_buffer: Rc<Buffer<'a, ArrayBufferType, AT>>,
    pub index_buffer: Rc<Buffer<'a, ElementArrayBufferType, IT>>,
    pub index_count: usize,
}

impl<'a, AT, IT> VertexBufferLite<'a, AT, IT> {
    /// Creates a VertexBufferLite,
    /// binds some data to the buffers,
    ///
    /// # Example
    /// ```
    /// # use gl::types::GLsizei;
    /// # use gl_thin::gl_fancy::{GPUState, VertexBufferBundle};
    /// # use gl_thin::gl_helper::GLErrorWrapper;
    /// # fn x(gpu_state: &mut GPUState, xyzuv:&[f32], indices:&[u16], program: &FlatColorShader) {
    /// let vbb = VertexBufferLite::new(
    ///                 gpu_state,
    ///                 xyzuv.into(),
    ///                 indices.into(),
    ///             )?;
    /// # }
    ///```
    pub fn new(
        gpu_state: &mut GPUState,
        vertex_data: BufferOwnership<'a, AT>,
        index_data: BufferOwnership<'a, IT>,
    ) -> Result<Self, GLErrorWrapper> {
        let index_count = index_data.as_slice().len();

        let mut vertex_buffer = Buffer::new()?;
        vertex_buffer.bound(gpu_state)?.load_any(vertex_data)?;
        let mut index_buffer = Buffer::new()?;
        index_buffer.bound(gpu_state)?.load_any(index_data)?;

        Ok(Self {
            vertex_buffer: Rc::new(vertex_buffer),
            index_buffer: Rc::new(index_buffer),
            index_count,
        })
    }
}

//

/// Use this struct to store buffers needed to render geometry.
/// The vertex_array stores bindings from attributes to buffers and is shader-specific
/// The vertex_buffer and index_buffer can be reused by multiple entities.
///
/// # See also
///
/// [VertexBufferLite]
pub struct VertexBufferBundle<'a, AT, IT> {
    pub vertex_array: VertexArray,
    pub vertex_buffer: Rc<Buffer<'a, ArrayBufferType, AT>>,
    pub index_buffer: Rc<Buffer<'a, ElementArrayBufferType, IT>>,
    pub index_count: usize,
}

impl<'a, AT, IT> VertexBufferBundle<'a, AT, IT> {
    pub fn incomplete() -> Result<Self, GLErrorWrapper> {
        Ok(Self {
            vertex_array: VertexArray::incomplete()?,
            vertex_buffer: Rc::new(Buffer::new()?),
            index_buffer: Rc::new(Buffer::new()?),
            index_count: 0,
        })
    }

    pub fn bind(
        &'a self,
        gpu_state: &'a mut GPUState,
    ) -> Result<BoundBuffers<'a, AT, IT>, GLErrorWrapper> {
        gpu_state.bind_vertex_array_and_buffers(
            &self.vertex_array,
            &self.vertex_buffer,
            &self.index_buffer,
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
        let index_count = index_data.as_slice().len();

        let mut vertex_buffer = Buffer::new()?;
        vertex_buffer.bound(gpu_state)?.load_any(vertex_data)?;
        let mut index_buffer = Buffer::new()?;
        index_buffer.bound(gpu_state)?.load_any(index_data)?;

        let vao = VertexArray::incomplete()?;
        vao.bound::<AT>(gpu_state)?
            .rig_multi_attributes(vertex_data_stride, attributes)?;

        Ok(Self {
            vertex_array: vao,
            vertex_buffer: Rc::new(vertex_buffer),
            index_buffer: Rc::new(index_buffer),
            index_count,
        })
    }

    pub fn from_buffers<'i>(
        gpu_state: &mut GPUState,
        buffers: &VertexBufferLite<'a, AT, IT>,
        vertex_data_stride: GLsizei,
        attributes: impl IntoIterator<Item = &'i (GLuint, GLint, GLsizei)>,
    ) -> Result<Self, GLErrorWrapper> {
        let vao = VertexArray::incomplete()?;
        vao.bound::<AT>(gpu_state)?
            .rig_multi_attributes(vertex_data_stride, attributes)?;

        Ok(Self {
            vertex_array: vao,
            vertex_buffer: buffers.vertex_buffer.clone(),
            index_buffer: buffers.index_buffer.clone(),
            index_count: buffers.index_count,
        })
    }

    pub fn reuse<'i>(
        &self,
        gpu_state: &mut GPUState,
        vertex_data_stride: GLsizei,
        attributes: impl IntoIterator<Item = &'i (GLuint, GLint, GLsizei)>,
    ) -> Result<Self, GLErrorWrapper> {
        let vao = VertexArray::incomplete()?;
        vao.bound::<AT>(gpu_state)?
            .rig_multi_attributes(vertex_data_stride, attributes)?;

        Ok(Self {
            vertex_array: vao,
            vertex_buffer: self.vertex_buffer.clone(),
            index_buffer: self.index_buffer.clone(),
            index_count: self.index_count,
        })
    }
}

//

#[derive(Copy, Clone)]
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

/// still experimental
pub struct BoundVertexArray<'a, 'g, AT> {
    pub vao: &'a VertexArray,
    pub gpu_state: &'g GPUState,
    phantom_data: PhantomData<AT>,
}

impl<'a, 'g, AT: GLBufferType> BoundVertexArray<'a, 'g, AT> {
    pub(crate) fn new(
        vao: &'a VertexArray,
        gpu_state: &'g mut GPUState,
    ) -> Result<Self, GLErrorWrapper> {
        vao.bind()?;
        Ok(Self {
            vao,
            gpu_state,
            phantom_data: Default::default(),
        })
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

impl<'a, 'g, AT> Drop for BoundVertexArray<'a, 'g, AT> {
    fn drop(&mut self) {
        unsafe {
            gl::BindVertexArray(0);
        }
        let _ = explode_if_gl_error();
    }
}
