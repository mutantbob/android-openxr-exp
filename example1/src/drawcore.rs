use crate::scene::MyScene;
use crate::xr_input::XrInputs;
use crate::Drawable;
use gl::types::GLsizei;
use gl_thin::errors::XrErrorWrapped;
use gl_thin::gl_fancy::GPUState;
use gl_thin::gl_helper::{explode_if_gl_error, FrameBuffer, GLErrorWrapper, Texture};
use gl_thin::linear::{
    xr_matrix4x4f_create_translation_rotation_scale, xr_matrix4x4f_invert_rigid_body, XrMatrix4x4f,
    XrQuaternionf, XrVector3f,
};
use gl_thin::openxr_helpers::{Backend, OpenXRComponent};
use glutin::config::{ConfigTemplate, ConfigTemplateBuilder, GlConfig};
use glutin::context::{AsRawContext, ContextAttributesBuilder, RawContext};
use glutin::display::{AsRawDisplay, Display, DisplayApiPreference, GlDisplay, RawDisplay};
use log::debug;
use openxr::{Graphics, OpenGlEs, SpaceLocation, View, ViewConfigurationView};
use openxr_sys::{Time, ViewConfigurationType};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawWindowHandle};
use std::error::Error;
use std::ffi::c_void;
use winit::event_loop::EventLoopWindowTarget;

//

pub struct FrameEnv {
    pub frame_buffer: FrameBuffer,
    pub depth_buffer: Texture,
}

impl FrameEnv {
    pub fn new(width: u32, height: u32) -> Result<Self, GLErrorWrapper> {
        Ok(Self {
            frame_buffer: FrameBuffer::new()?,
            depth_buffer: Texture::depth_buffer(width as i32, height as i32)?,
        })
    }

    /// bind the frame_buffer, and attach the color_buffer (parameter) and the depth_buffer (field)
    pub fn prepare_to_draw(
        &self,
        color_buffer: &Texture,
        width: u32,
        height: u32,
    ) -> Result<(), GLErrorWrapper> {
        self.frame_buffer.bind()?;
        color_buffer.attach(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0)?;
        self.depth_buffer
            .attach(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, 0)?;

        unsafe { gl::Viewport(0, 0, width as GLsizei, height as GLsizei) }; // XXX
        explode_if_gl_error()?;

        if gl::DrawBuffer::is_loaded() {
            unsafe { gl::DrawBuffer(gl::COLOR_ATTACHMENT0) };
            explode_if_gl_error()?;
        }
        Ok(())
    }
}

//

pub fn skybox_view_matrix(rotation: &XrQuaternionf) -> XrMatrix4x4f {
    let scale = XrVector3f::default_scale();
    let view_matrix = xr_matrix4x4f_create_translation_rotation_scale(
        &XrVector3f::default_translation(),
        rotation,
        &scale,
    );
    xr_matrix4x4f_invert_rigid_body(&view_matrix)
}

pub struct ActiveRenderer {
    pub frame_env: FrameEnv,
    pub scene: MyScene,
    pub openxr: OpenXRComponent<openxr::OpenGlEs>,
    pub gpu_state: GPUState,

    inputs: XrInputs,
}

impl Drawable for ActiveRenderer {
    fn handle_events_and_draw(&mut self) {
        // The event handling loop should probably be more sophisticated than this.
        self.openxr.poll_till_no_events().unwrap();

        //

        match self.draw_inner() {
            Ok(_) => {}
            Err(e) => {
                log::error!("malfunction during draw_inner() {}", e);
            }
        };
    }

    fn suspend(&mut self) {
        self.openxr.xr_session.request_exit().unwrap();
    }
}

impl ActiveRenderer {
    /// Create template to find OpenGL config.
    pub fn config_template(raw_window_handle: RawWindowHandle) -> ConfigTemplate {
        let builder = ConfigTemplateBuilder::new()
            //.with_alpha_size(8)
            .compatible_with_native_window(raw_window_handle);

        #[cfg(cgl_backend)]
        let builder = builder.with_transparency(true).with_multisampling(8);

        builder.build()
    }

    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>) -> Result<Self, Box<dyn Error>> {
        let (display_ptr, raw_context) = Self::build_android_egl_context(event_loop)?;

        let mut gpu_state = GPUState::new();

        let openxr =
            OpenXRComponent::new_android(display_ptr as *mut c_void, raw_context as *mut c_void)?;

        let vcv0 = openxr.view_config_views[0];
        let frame_env = FrameEnv::new(
            vcv0.recommended_image_rect_width,
            vcv0.recommended_image_rect_height,
        )?;
        let scene = MyScene::new(&mut gpu_state)?;

        let inputs = XrInputs::new(&openxr.xr_instance, &openxr.xr_session)?;

        Ok(Self {
            frame_env,
            scene,
            openxr,
            gpu_state,
            inputs,
        })
    }

    pub fn build_android_egl_context<T>(
        event_loop: &EventLoopWindowTarget<T>,
    ) -> Result<(*const c_void, *const c_void), Box<dyn Error>> {
        let raw_display = event_loop.raw_display_handle();

        let Display::Egl(glutin_display) =
            unsafe { glutin::display::Display::new(raw_display, DisplayApiPreference::Egl) }?;

        let RawDisplay::Egl(display_ptr) = glutin_display.raw_display();

        let window = winit::window::Window::new(event_loop)?;
        let raw_window_handle = window.raw_window_handle();

        let template = Self::config_template(raw_window_handle);

        let config = unsafe {
            let configs_list: Vec<_> = glutin_display.find_configs(template)?.collect();
            if true {
                debug!("glutin display configs [{}]", configs_list.len());
                for config in &configs_list {
                    debug!("config {:?}", config.config_surface_types());
                }
            }
            configs_list
                .into_iter()
                .reduce(|accum, config| {
                    // Find the config with the maximum number of samples.
                    //
                    // In general if you're not sure what you want in template you can request or
                    // don't want to require multisampling for example, you can search for a
                    // specific option you want afterwards.
                    //
                    // XXX however on macOS you can request only one config, so you should do
                    // a search with the help of `find_configs` and adjusting your template.
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        };

        let context = {
            let attr = ContextAttributesBuilder::new().build(Some(raw_window_handle));
            unsafe { glutin_display.create_context(&config, &attr) }
        }?;

        let context = context.make_current_surfaceless()?;

        let RawContext::Egl(raw_context) = context.raw_context();
        Ok((display_ptr, raw_context))
    }

    /// iterate through the various OpenXR views and paint them
    pub fn draw_inner(&mut self) -> Result<(), XrErrorWrapped> {
        let gpu_state = &mut self.gpu_state;

        let before_paint = |openxr: &OpenXRComponent<OpenGlEs>,
                            frame_state: &openxr::FrameState| {
            self.inputs.sync_actions(&openxr.xr_session).unwrap();

            let location = self.inputs.controller_1_locate_if_active(
                &openxr.xr_session,
                &openxr.xr_space,
                frame_state.predicted_display_time,
            );

            if false {
                debug!("space location {:?}", location.map(|sl| sl.pose));
            }
            (location, gpu_state)
        };

        let lambda =
            |view_i: &View,
             vcv: &ViewConfigurationView,
             predicted_display_time,
             &render_destination: &u32,
             // gpu_state: &mut GPUState,
             (controller_1, gpu_state): &mut (Option<SpaceLocation>, &mut GPUState)| {
                Self::paint_one_view(
                    view_i,
                    vcv,
                    predicted_display_time,
                    &self.scene,
                    &self.frame_env,
                    render_destination,
                    gpu_state,
                    controller_1,
                )
                .unwrap();
            };
        let after_paint = |_: &OpenXRComponent<OpenGlEs>, _: &openxr::FrameState, _| {};

        self.openxr.paint_vr_multiview(
            before_paint,
            lambda,
            after_paint,
            ViewConfigurationType::PRIMARY_STEREO,
            // &mut self.gpu_state,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_one_view(
        view_i: &View,
        view_config_view: &ViewConfigurationView,
        time: Time,
        renderer: &MyScene,
        frame_env: &FrameEnv,
        color_buffer: <Backend as Graphics>::SwapchainImage,
        gpu_state: &mut GPUState,
        controller_1: &Option<SpaceLocation>,
    ) -> Result<(), Box<dyn Error>> {
        let width = view_config_view.recommended_image_rect_width;
        let height = view_config_view.recommended_image_rect_height;
        frame_env.prepare_to_draw(&Texture::borrowed(color_buffer), width, height)?;
        renderer.draw(
            &view_i.fov.into(),
            &view_i.pose.orientation.into(),
            &view_i.pose.position.into(),
            time,
            gpu_state,
            controller_1,
        )?;

        Ok(())
    }
}

pub fn debug_string_matrix(matrix: &XrMatrix4x4f) -> String {
    let matrix = matrix.slice();
    format!(
        "{:?}\n{:?}\n{:?}\n{:?}",
        &matrix[0..4],
        &matrix[4..8],
        &matrix[8..12],
        &matrix[12..16]
    )
}
