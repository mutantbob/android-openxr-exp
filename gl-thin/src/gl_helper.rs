use crate::gl_fancy::{BoundTexture, BoundVertexArray, GPUState, OneBoundBuffer};
use gl::types::{GLchar, GLenum, GLfloat, GLint, GLsizei, GLsizeiptr, GLuint, GLushort};
use std::ffi::{c_void, CString};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem::{size_of, MaybeUninit};
use std::ptr::null;

pub fn initialize_gl_using_egli() {
    gl::load_with(|name| {
        let name = CString::new(name).unwrap();
        (unsafe { egli::ffi::eglGetProcAddress(name.as_ptr()) }) as *mut _
    });
}

pub fn explode_if_gl_error() -> Result<(), GLErrorWrapper> {
    let mut last_err = None;
    loop {
        let err = unsafe { gl::GetError() };
        if err == gl::NO_ERROR {
            break;
        } else {
            last_err = Some(err);
        }
    }

    match last_err {
        Some(e) => Err(GLErrorWrapper::new(e)),
        None => Ok(()),
    }
}

//

#[derive(Clone)]
pub enum MessageForError {
    None,
    CStr(CString),
    Str(String),
}

#[derive(Clone)]
pub struct GLErrorWrapper {
    pub code: GLenum,
    pub message: MessageForError,
}

impl GLErrorWrapper {
    pub fn with_message(msg: CString) -> Self {
        Self {
            code: 0,
            message: MessageForError::CStr(msg),
        }
    }

    pub fn with_message2(msg: String) -> Self {
        Self {
            code: 0,
            message: MessageForError::Str(msg),
        }
    }

    pub fn new(code: GLenum) -> Self {
        Self {
            code,
            message: MessageForError::None,
        }
    }
}

impl Debug for GLErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            MessageForError::CStr(msg) => write!(f, "{:?}", msg),
            MessageForError::Str(msg) => write!(f, "{:?}", msg),
            MessageForError::None => write!(f, "0x{:x}", self.code),
        }
    }
}

impl Display for GLErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl std::error::Error for GLErrorWrapper {}

//

pub enum Ownership<T> {
    Borrowed(T),
    Owned(T),
    None,
}

impl<T> Ownership<T> {
    pub fn unwrap(&self) -> &T {
        match self {
            Ownership::Borrowed(x) | Ownership::Owned(x) => x,
            Ownership::None => {
                panic!("Ownership::None")
            }
        }
    }
}

//

pub trait BufferTarget {
    const TARGET: GLenum;
}

pub struct ArrayBufferType {}
impl BufferTarget for ArrayBufferType {
    const TARGET: GLenum = gl::ARRAY_BUFFER;
}

pub struct ElementArrayBufferType {}
impl BufferTarget for ElementArrayBufferType {
    const TARGET: GLenum = gl::ELEMENT_ARRAY_BUFFER;
}

//

pub struct VertexArray(GLuint);

impl VertexArray {
    pub fn incomplete() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenVertexArrays(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;
        Ok(Self(unsafe { rval.assume_init() }))
    }

    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::BindVertexArray(self.0) }
        explode_if_gl_error()
    }

    pub fn bound<'a, 'g, AT: GLBufferType>(
        &'a self,
        gpu_state: &'g mut GPUState,
    ) -> Result<BoundVertexArray<'a, 'g, AT>, GLErrorWrapper> {
        BoundVertexArray::new(self, gpu_state)
    }

    pub fn borrow_raw(&self) -> GLuint {
        self.0
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe { gl::DeleteVertexArrays(1, &self.0) }
    }
}

//

pub enum BufferOwnership<'a, T> {
    Reference(&'a [T]),
    Owned(Vec<T>),
    None,
}

impl<'a, T> BufferOwnership<'a, T> {
    pub fn as_slice<'b: 'a>(&'b self) -> &'a [T] {
        match self {
            BufferOwnership::Reference(slice) => slice,
            BufferOwnership::Owned(vec) => vec.as_slice(),
            BufferOwnership::None => panic!("called as_slice() on None"),
        }
    }
}

impl<'a, T> From<&'a [T]> for BufferOwnership<'a, T> {
    fn from(value: &'a [T]) -> Self {
        BufferOwnership::Reference(value)
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for BufferOwnership<'a, T> {
    fn from(value: &'a [T; N]) -> Self {
        BufferOwnership::Reference(value)
    }
}

impl<'a, T> From<Vec<T>> for BufferOwnership<'a, T> {
    fn from(value: Vec<T>) -> Self {
        BufferOwnership::Owned(value)
    }
}

//

pub struct Buffer<'a, B, T> {
    handle: GLuint,
    data: BufferOwnership<'a, T>,
    phantom_data: PhantomData<B>,
}

impl<'a, B, T> Buffer<'a, B, T> {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenBuffers(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;

        Ok(Buffer {
            handle: unsafe { rval.assume_init() },
            data: BufferOwnership::None,
            phantom_data: Default::default(),
        })
    }
}

impl<'a, B, T> Drop for Buffer<'a, B, T> {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.handle) }
    }
}

impl<'a, B: BufferTarget, T> Buffer<'a, B, T> {
    pub fn bound<'g, 's>(
        &'s mut self,
        gpu_state: &'g mut GPUState,
    ) -> Result<OneBoundBuffer<'s, 'g, 'a, B, T>, GLErrorWrapper> {
        OneBoundBuffer::new(gpu_state, self)
    }

    /// # Safety
    /// assumes that the buffer has been bound using [gl::BindBuffer]
    pub unsafe fn load_any(&mut self, value: BufferOwnership<'a, T>) -> Result<(), GLErrorWrapper> {
        self.data = value;
        let slice = self.data.as_slice();
        let byte_count: GLsizeiptr = slice.len() as GLsizeiptr * size_of::<T>() as GLsizeiptr;
        unsafe {
            gl::BufferData(
                B::TARGET,
                byte_count,
                slice.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            )
        }
        explode_if_gl_error()
    }

    pub fn load(&mut self, values: &'a [T]) -> Result<(), GLErrorWrapper> {
        self.bind()?;
        let byte_count: GLsizeiptr = values.len() as GLsizeiptr * size_of::<T>() as GLsizeiptr;
        unsafe {
            gl::BufferData(
                B::TARGET,
                byte_count,
                values.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            )
        }
        self.data = BufferOwnership::Reference(values);
        explode_if_gl_error()
    }

    pub fn load_owned(&mut self, values: Vec<T>) -> Result<(), GLErrorWrapper> {
        self.bind()?; // XXX move this method to a new BoundBuffer type
        let byte_count: GLsizeiptr = values.len() as GLsizeiptr * size_of::<T>() as GLsizeiptr;
        unsafe {
            gl::BufferData(
                B::TARGET,
                byte_count,
                values.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            )
        }
        self.data = BufferOwnership::Owned(values);
        explode_if_gl_error()
    }

    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::BindBuffer(B::TARGET, self.handle) };
        explode_if_gl_error()
    }

    pub fn borrow_raw(&self) -> GLuint {
        self.handle
    }
}

//

pub trait ShaderFlavor {
    const FLAVOR: GLenum;
}

pub struct VertexShader {}
impl ShaderFlavor for VertexShader {
    const FLAVOR: GLenum = gl::VERTEX_SHADER;
}

pub struct FragmentShader {}
impl ShaderFlavor for FragmentShader {
    const FLAVOR: GLenum = gl::FRAGMENT_SHADER;
}

//

pub struct Shader<T> {
    handle: Option<GLuint>,
    phantom_data: PhantomData<T>,
}

impl<F: ShaderFlavor> Shader<F> {
    pub fn new_raw() -> Result<Self, GLErrorWrapper> {
        let rval = unsafe { gl::CreateShader(F::FLAVOR) };
        explode_if_gl_error()?;
        Ok(Self {
            handle: Some(rval),
            phantom_data: Default::default(),
        })
    }

    pub fn compile(vertex_shader: impl AsRef<str>) -> Result<Self, GLErrorWrapper> {
        let rval = Self::new_raw()?;
        let string = vertex_shader.as_ref();
        let bytes = string.as_bytes();
        let strings = [bytes.as_ptr() as *const GLchar];
        let lengths = [bytes.len() as GLint];
        unsafe { gl::ShaderSource(rval.borrow(), 1, strings.as_ptr(), lengths.as_ptr()) };
        explode_if_gl_error()?;
        unsafe { gl::CompileShader(rval.borrow()) };
        explode_if_gl_error()?;

        let mut is_compiled = 0;
        unsafe { gl::GetShaderiv(rval.borrow(), gl::COMPILE_STATUS, &mut is_compiled) };
        if is_compiled == 0 {
            let message = rval.get_shader_info_log();
            Err(GLErrorWrapper::with_message(message))
        } else {
            Ok(rval)
        }
    }
}

impl<F> Shader<F> {
    /// get access to the GL handle in case you need to call some low-level stuff
    pub fn borrow(&self) -> GLuint {
        self.handle.unwrap()
    }

    #[must_use]
    /// take ownership of the GL handle inside this object.  You are now responsible for calling gl::DeleteShader
    pub fn unmanage(mut self) -> GLuint {
        self.handle.take().unwrap()
    }

    pub fn get_shader_info_log(&self) -> CString {
        let mut max_length = 0;
        unsafe { gl::GetShaderiv(self.borrow(), gl::INFO_LOG_LENGTH, &mut max_length) };
        let mut error_log = Vec::with_capacity(max_length as usize);
        unsafe {
            gl::GetShaderInfoLog(
                self.borrow(),
                max_length,
                &mut max_length,
                error_log.as_mut_ptr(),
            );
            error_log.set_len(max_length as usize);
        }
        CString::new(from_glchar_to_u8(error_log)).unwrap()
    }
}

impl<F> Drop for Shader<F> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe { gl::DeleteShader(handle) }
        }
    }
}

//

pub struct Program(GLuint);

impl Program {
    pub fn new_empty() -> Result<Self, GLErrorWrapper> {
        let rval = unsafe { gl::CreateProgram() };
        explode_if_gl_error()?;
        Ok(Self(rval))
    }

    pub fn compile(
        vertex_shader: impl AsRef<str>,
        fragment_shader: impl AsRef<str>,
    ) -> Result<Self, GLErrorWrapper> {
        let vertex_shader = Shader::<VertexShader>::compile(vertex_shader.as_ref())?;
        let fragment_shader = Shader::<FragmentShader>::compile(fragment_shader.as_ref())?;

        let mut rval = Self::new_empty().unwrap();
        rval.attach(&vertex_shader).unwrap();
        rval.attach(&fragment_shader).unwrap();

        unsafe { gl::LinkProgram(rval.borrow()) };
        explode_if_gl_error().unwrap();

        let mut link_status = 0;
        unsafe { gl::GetProgramiv(rval.borrow(), gl::LINK_STATUS, &mut link_status) };
        explode_if_gl_error().unwrap();
        if link_status == 0 {
            return Err(GLErrorWrapper::with_message(rval.get_program_info_log()));
        }

        rval.detach(&vertex_shader);
        rval.detach(&fragment_shader);

        Ok(rval)
    }

    pub fn borrow(&self) -> GLuint {
        self.0
    }

    pub fn take_ownership(handle: GLuint) -> Self {
        Self(handle)
    }

    fn attach<T>(&mut self, shader: &Shader<T>) -> Result<(), GLErrorWrapper> {
        unsafe { gl::AttachShader(self.borrow(), shader.borrow()) };
        explode_if_gl_error()
    }

    fn detach<T>(&mut self, shader: &Shader<T>) {
        unsafe { gl::DetachShader(self.borrow(), shader.borrow()) };
    }

    pub fn use_(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::UseProgram(self.0) }
        explode_if_gl_error()
    }

    pub fn get_uniform_location(&self, name: &str) -> Result<GLuint, GLErrorWrapper> {
        let c_name = CString::new(name).unwrap();
        let rval = unsafe { gl::GetUniformLocation(self.0, c_name.as_ptr() as *const GLchar) };
        explode_if_gl_error()?;
        if rval < 0 {
            return Err(GLErrorWrapper::with_message(
                CString::new(format!("no attribute named {}", name)).unwrap(),
            ));
        }
        Ok(rval as GLuint)
    }

    pub fn get_attribute_location(&self, p0: &str) -> Result<GLuint, GLErrorWrapper> {
        let name = CString::new(p0).unwrap();
        let rval = unsafe { gl::GetAttribLocation(self.0, name.as_ptr()) };
        explode_if_gl_error()?;
        if rval < 0 {
            panic!("no attribute named {} on this program", p0)
        } else {
            Ok(rval as GLuint)
        }
    }

    //

    pub fn set_uniform_1i(&self, location: GLint, v0: GLint) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform1i(location, v0) }
        explode_if_gl_error()
    }

    pub fn set_uniform_1f(&self, location: GLint, v0: GLfloat) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform1f(location, v0) }
        explode_if_gl_error()
    }

    pub fn set_uniform_2f(
        &self,
        location: GLint,
        v0: GLfloat,
        v1: GLfloat,
    ) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform2f(location, v0, v1) }
        explode_if_gl_error()
    }

    pub fn set_uniform_2fv(
        &self,
        location: GLint,
        val: &[GLfloat; 2],
    ) -> Result<(), GLErrorWrapper> {
        // Uniform2fv has failed me in the past
        unsafe { gl::Uniform2f(location, val[0], val[1]) }
        explode_if_gl_error()
    }

    pub fn set_uniform_3f(&self, name: &str, x: f32, y: f32, z: f32) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform3f(self.get_uniform_location(name)? as GLint, x, y, z) }
        explode_if_gl_error()
    }

    pub fn set_uniform_4f(
        &self,
        location: GLint,
        x: f32,
        y: f32,
        z: f32,
        a: f32,
    ) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform4f(location, x, y, z, a) }
        explode_if_gl_error()
    }

    pub fn set_mat4(&self, location: GLint, val: &[[f32; 4]; 4]) -> Result<(), GLErrorWrapper> {
        unsafe { gl::UniformMatrix4fv(location, 1, 0, val[0].as_ptr()) }
        explode_if_gl_error()
    }

    pub fn set_mat4u(&self, location: GLint, val: &[f32; 16]) -> Result<(), GLErrorWrapper> {
        unsafe { gl::UniformMatrix4fv(location, 1, 0, val.as_ptr()) }
        explode_if_gl_error()
    }

    pub fn get_program_info_log(&self) -> CString {
        let mut max_length = 0;
        unsafe { gl::GetProgramiv(self.borrow(), gl::INFO_LOG_LENGTH, &mut max_length) };
        let mut error_log = Vec::with_capacity(max_length as usize);
        unsafe {
            gl::GetProgramInfoLog(
                self.borrow(),
                max_length,
                &mut max_length,
                error_log.as_mut_ptr(),
            );
            error_log.set_len(max_length as usize);
        }
        CString::new(from_glchar_to_u8(error_log)).unwrap()
    }
}

fn from_glchar_to_u8(src: Vec<GLchar>) -> Vec<u8> {
    src.into_iter().map(|x| x as u8).collect::<Vec<_>>()
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.0) }
    }
}

//

pub struct FrameBuffer(GLuint);

impl FrameBuffer {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenFramebuffers(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;
        Ok(Self(unsafe { rval.assume_init() }))
    }
    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.0) }
        explode_if_gl_error()
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe { gl::DeleteFramebuffers(1, &self.0) };
    }
}

//

pub struct Texture(pub Ownership<GLuint>);

impl Texture {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenTextures(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;
        Ok(Self(Ownership::Owned(unsafe { rval.assume_init() })))
    }

    pub fn borrowed(handle: GLuint) -> Self {
        Self(Ownership::Borrowed(handle))
    }

    pub fn depth_buffer(
        width: i32,
        height: i32,
        gpu_state: &mut GPUState,
    ) -> Result<Self, GLErrorWrapper> {
        let rval = Self::new()?;

        let target = gl::TEXTURE_2D;

        rval.bound(target, gpu_state)?.configure::<GLuint>(
            0,
            gl::DEPTH_COMPONENT24 as i32,
            width,
            height,
            0,
            gl::DEPTH_COMPONENT,
        )?;

        Ok(rval)
    }

    pub fn bound<'g, 't>(
        &'t self,
        target: GLenum,
        gpu_state: &'g mut GPUState,
    ) -> Result<BoundTexture<'g, 't>, GLErrorWrapper> {
        BoundTexture::new(gpu_state, self, target)
    }

    pub fn borrow(&self) -> GLuint {
        match &self.0 {
            Ownership::Borrowed(val) | Ownership::Owned(val) => *val,
            Ownership::None => panic!("no value, how did we get into this state?"),
        }
    }

    /// bind before calling this, and don't forget to make the mipmaps;
    /// or just call write_pixels_and_generate_mipmap()
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn configure<T: GLBufferType>(
        &self,
        target: GLenum,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: GLenum,
    ) -> Result<(), GLErrorWrapper> {
        unsafe {
            gl::TexImage2D(
                target,
                level,
                internal_format,
                width,
                height,
                border,
                format,
                // the call can crash if you pass the wrong value for type
                T::TYPE_CODE,
                null(),
            )
        };
        explode_if_gl_error()
    }

    /// Consider using BoundTexture instead
    pub fn bind(&self, target: GLenum) -> Result<(), GLErrorWrapper> {
        unsafe { gl::BindTexture(target, *self.0.unwrap()) };
        explode_if_gl_error()
    }

    pub fn attach(
        &self,
        target: GLenum,
        attachment: GLenum,
        tex_target: GLenum,
        level: i32,
    ) -> Result<(), GLErrorWrapper> {
        let texture = *self.0.unwrap();
        unsafe { gl::FramebufferTexture2D(target, attachment, tex_target, texture, level) };
        explode_if_gl_error()
    }

    #[deprecated]
    pub fn get_width(&self) -> Result<GLint, GLErrorWrapper> {
        self.bind(gl::TEXTURE_2D)?;

        let mut rval = 0;
        unsafe { gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_WIDTH, &mut rval) };
        explode_if_gl_error()?;

        Ok(rval)
    }

    #[deprecated]
    pub fn get_height(&self) -> Result<GLint, GLErrorWrapper> {
        self.bind(gl::TEXTURE_2D)?;

        let mut rval = 0;
        unsafe { gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_HEIGHT, &mut rval) };
        explode_if_gl_error()?;

        Ok(rval)
    }

    #[deprecated]
    pub fn get_dimensions(&self) -> Result<(GLint, GLint), GLErrorWrapper> {
        self.bind(gl::TEXTURE_2D)?;

        let mut width = 0;
        unsafe { gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_WIDTH, &mut width) };
        explode_if_gl_error()?;

        let mut height = 0;
        unsafe { gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_HEIGHT, &mut height) };
        explode_if_gl_error()?;

        Ok((width, height))
    }

    #[deprecated]
    #[allow(deprecated)]
    pub fn write_pixels_and_generate_mipmap<T: GLBufferType>(
        &mut self,
        target: GLenum,
        level: GLint,
        internal_format: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        pixels: &[T],
    ) -> Result<(), GLErrorWrapper> {
        self.write_pixels(
            target,
            level,
            internal_format,
            width,
            height,
            format,
            pixels,
        )?;
        unsafe { self.generate_mipmap() }
    }

    #[deprecated]
    /// Remember to populate the mipmap by either writing all the different mipmap `level`s or using `self.generate_mipmap()`
    pub fn write_pixels<T: GLBufferType>(
        &mut self,
        target: GLenum,
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
        self.bind(target)?;
        unsafe {
            gl::TexImage2D(
                target,
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

    /// # Safety
    /// did you `bind()` this texture yet?
    pub unsafe fn generate_mipmap(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::GenerateMipmap(gl::TEXTURE_2D) };
        explode_if_gl_error()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        match self.0 {
            Ownership::Owned(handle) => unsafe { gl::DeleteTextures(1, &handle) },
            Ownership::Borrowed(_) | Ownership::None => {}
        }
    }
}

//

pub struct TextureWithTarget {
    pub texture: Texture,
    pub target: GLenum,
}

impl TextureWithTarget {
    pub fn new(texture: Texture, target: GLenum) -> Self {
        Self { texture, target }
    }

    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        self.texture.bind(self.target)
    }

    pub fn is_texture_2d(&self) -> bool {
        self.target == gl::TEXTURE_2D
    }
}

//

pub trait GLBufferType {
    const TYPE_CODE: GLenum;
}

impl GLBufferType for GLfloat {
    const TYPE_CODE: GLenum = gl::FLOAT;
}

impl GLBufferType for u8 {
    const TYPE_CODE: GLenum = gl::UNSIGNED_BYTE;
}

impl GLBufferType for GLushort {
    const TYPE_CODE: GLenum = gl::UNSIGNED_SHORT;
}

impl GLBufferType for GLuint {
    const TYPE_CODE: GLenum = gl::UNSIGNED_INT;
}

/// # Safety
/// The "pointer" returned by this function is really just a byte offset (delta).
/// The OpenGL API is dumb like that.
/// Do not try to dereference it.
/// It is only good for calls to functions like gl::VertexAttribPointer and gl::DrawArrays
pub const unsafe fn gl_offset_for<T>(count: GLsizei) -> *const c_void {
    (count * size_of::<T>() as GLsizei) as *const c_void
}

pub fn bytes_per_pixel<T: GLBufferType>(format: GLenum) -> Result<usize, GLErrorWrapper> {
    let alpha = match format {
        gl::RGB => 3,
        gl::RED => 1,
        gl::RGBA => 4,
        _ => {
            // there are so many variants I am missing ...
            return Err(GLErrorWrapper::with_message2(format!(
                "unhandled format 0x{:x}",
                format
            )));
        }
    };

    Ok(alpha * size_of::<T>())
}
