use log::debug;
use openxr::{Graphics, Instance, Swapchain};
use openxr_sys::{
    create_session, GraphicsBindingOpenGLESAndroidKHR, GraphicsRequirementsOpenGLESKHR,
    SessionCreateInfo, SwapchainImageOpenGLESKHR, SystemId,
};
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

pub struct AndroidGLESCreateInfo {
    /// You probably want gst_gl_context_get_gl_context()
    pub gl_context: *mut c_void,
    /// You probably want gst_gl_display_get_handle()
    pub gl_display: *mut c_void,
    pub system_id: SystemId,
}

//

/// I wish openxr published this function
pub fn cvt(x: openxr_sys::Result) -> openxr::Result<openxr_sys::Result> {
    if x.into_raw() >= 0 {
        Ok(x)
    } else {
        Err(x)
    }
}

/// I wish openxr published this function
fn get_arr_init<T: Copy>(
    init: T,
    mut getter: impl FnMut(u32, &mut u32, *mut T) -> openxr_sys::Result,
) -> openxr::Result<Vec<T>> {
    let mut output = 0;
    cvt(getter(0, &mut output, std::ptr::null_mut()))?;
    let mut buffer = vec![init; output as usize];
    loop {
        match cvt(getter(output, &mut output, buffer.as_mut_ptr() as _)) {
            Ok(_) => {
                buffer.truncate(output as usize);
                return Ok(buffer);
            }
            Err(openxr_sys::Result::ERROR_SIZE_INSUFFICIENT) => {
                buffer.resize(output as usize, init);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

//

pub struct AndroidOpenGLES {}

impl Graphics for AndroidOpenGLES {
    type Requirements = GraphicsRequirementsOpenGLESKHR;
    type SessionCreateInfo = AndroidGLESCreateInfo; // XXX this should be a little more specific
    type Format = i64;
    type SwapchainImage = u32;

    fn raise_format(format: i64) -> Self::Format {
        format
    }

    fn lower_format(format: Self::Format) -> i64 {
        format
    }

    fn requirements(
        instance: &Instance,
        system_id: SystemId,
    ) -> openxr::Result<Self::Requirements> {
        let mut graphics_requirements = GraphicsRequirementsOpenGLESKHR::out(null_mut());

        let get_open_gles_graphics_requirements = instance
            .exts()
            .khr_opengl_es_enable
            .unwrap()
            .get_open_gles_graphics_requirements;

        debug!(
            "get_open_gles_graphics_requirements {:?}",
            get_open_gles_graphics_requirements as usize
        );
        let result = unsafe {
            (get_open_gles_graphics_requirements)(
                instance.as_raw(),
                system_id,
                graphics_requirements.as_mut_ptr(),
            )
        };
        cvt(result)?;
        Ok(unsafe { graphics_requirements.assume_init() })
    }

    unsafe fn create_session(
        instance: &Instance,
        _system: SystemId,
        info: &Self::SessionCreateInfo,
    ) -> openxr::Result<openxr_sys::Session> {
        let graphics_binding = GraphicsBindingOpenGLESAndroidKHR {
            ty: GraphicsBindingOpenGLESAndroidKHR::TYPE,
            next: null(),
            config: null_mut(),
            context: info.gl_context,
            display: info.gl_display,
        };
        let session_create_info = SessionCreateInfo {
            ty: SessionCreateInfo::TYPE,
            next: &graphics_binding as *const _ as *const c_void,
            create_flags: Default::default(),
            system_id: info.system_id,
        };

        let mut rval = MaybeUninit::uninit();
        create_session(instance.as_raw(), &session_create_info, rval.as_mut_ptr());
        Ok(rval.assume_init())
    }

    fn enumerate_swapchain_images(
        swapchain: &Swapchain<Self>,
    ) -> openxr::Result<Vec<Self::SwapchainImage>> {
        let images = get_arr_init(
            SwapchainImageOpenGLESKHR {
                ty: openxr_sys::SwapchainImageOpenGLESKHR::TYPE,
                next: null_mut(),
                image: 0,
            },
            |capacity, count, buf| unsafe {
                (swapchain.instance().fp().enumerate_swapchain_images)(
                    swapchain.as_raw(),
                    capacity,
                    count,
                    buf as *mut _,
                )
            },
        )?;
        Ok(images.into_iter().map(|x| x.image).collect())
    }
}
