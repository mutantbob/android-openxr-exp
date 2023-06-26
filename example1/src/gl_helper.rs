use gl::types::{GLchar, GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
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
pub struct GLErrorWrapper {
    pub code: GLenum,
    pub message: Option<CString>,
}

impl GLErrorWrapper {
    pub fn with_message(msg: CString) -> Self {
        Self {
            code: 0,
            message: Some(msg),
        }
    }
}

impl GLErrorWrapper {
    pub fn new(code: GLenum) -> Self {
        Self {
            code,
            message: None,
        }
    }
}

impl Debug for GLErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.message.as_ref() {
            Some(msg) => write!(f, "{:?}", msg),
            None => write!(f, "0x{:x}", self.code),
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

pub trait BufferType {
    const TARGET: GLenum;
}

pub struct ArrayBufferType {}
impl BufferType for ArrayBufferType {
    const TARGET: GLenum = gl::ARRAY_BUFFER;
}

pub struct ElementArrayBufferType {}
impl BufferType for ElementArrayBufferType {
    const TARGET: GLenum = gl::ELEMENT_ARRAY_BUFFER;
}

//

pub struct VertexArray(GLuint);

impl VertexArray {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenVertexArrays(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;
        Ok(Self(unsafe { rval.assume_init() }))
    }

    pub fn bind(&self) -> Result<(), GLErrorWrapper> {
        unsafe { gl::BindVertexArray(self.0) }
        explode_if_gl_error()
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

pub struct Buffer<'a, B, T> {
    handle: GLuint,
    data: Option<&'a [T]>,
    phantom_data: PhantomData<B>,
}

impl<'a, B, T> Buffer<'a, B, T> {
    pub fn new() -> Result<Self, GLErrorWrapper> {
        let mut rval = MaybeUninit::uninit();
        unsafe { gl::GenBuffers(1, rval.as_mut_ptr()) };
        explode_if_gl_error()?;

        Ok(Buffer {
            handle: unsafe { rval.assume_init() },
            data: None,
            phantom_data: Default::default(),
        })
    }
}

impl<'a, B, T> Drop for Buffer<'a, B, T> {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.handle) }
    }
}

impl<'a, B: BufferType, T> Buffer<'a, B, T> {
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
        self.data = Some(values);
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
            )
        };
        CString::new(error_log).unwrap()
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
        let vertex_shader = Shader::<VertexShader>::compile(vertex_shader.as_ref()).unwrap();
        let fragment_shader = Shader::<FragmentShader>::compile(fragment_shader.as_ref()).unwrap();

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
    pub fn set_uniform_3f(&self, name: &str, x: f32, y: f32, z: f32) -> Result<(), GLErrorWrapper> {
        unsafe { gl::Uniform3f(self.get_uniform_location(name)? as GLint, x, y, z) }
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
            )
        };
        CString::new(error_log).unwrap()
    }
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

pub struct Texture(Ownership<GLuint>);

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

    pub fn depth_buffer(width: i32, height: i32) -> Result<Self, GLErrorWrapper> {
        let rval = Self::new()?;
        let target = gl::TEXTURE_2D;
        rval.bind(target)?;
        rval.configure(
            target,
            0,
            gl::DEPTH_COMPONENT24 as i32,
            width,
            height,
            0,
            gl::DEPTH_COMPONENT,
            gl::UNSIGNED_INT,
            None,
        )?;
        Ok(rval)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &self,
        target: GLenum,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: GLenum,
        type_: GLenum,
        pixels: Option<&[c_void]>,
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
                type_,
                match pixels {
                    None => null(),
                    Some(slice) => slice.as_ptr(),
                },
            )
        };
        explode_if_gl_error()
    }

    fn bind(&self, target: GLenum) -> Result<(), GLErrorWrapper> {
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
}

impl Drop for Texture {
    fn drop(&mut self) {
        match self.0 {
            Ownership::Owned(handle) => unsafe { gl::DeleteTextures(1, &handle) },
            Ownership::Borrowed(_) | Ownership::None => {}
        }
    }
}

/// # Safety
/// The "pointer" returned by this function is really just a byte offset (delta).
/// The OpenGL API is dumb like that.
/// Do not try to dereference it.
/// It is only good for calls to functions like gl::VertexAttribPointer and gl::DrawArrays
pub const unsafe fn gl_offset_for<T>(count: GLsizei) -> *const c_void {
    (count * size_of::<T>() as GLsizei) as *const c_void
}
