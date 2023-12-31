use crate::errors::{Wrappable, XrErrorWrapped};
use gl::types::GLint;
use itertools::izip;
use log::{debug, error, info, warn};
use openxr::sys::{result_to_string, Result as XrResult, MAX_RESULT_STRING_SIZE};
use openxr::OpenGlEs;
use openxr::{
    ActionSet, ApplicationInfo, Binding, CompositionLayerBase, CompositionLayerProjection, Entry,
    Event, EventDataBuffer, ExtensionSet, FormFactor, FrameState, FrameStream, FrameWaiter,
    Graphics, Instance, Posef, Quaternionf, ReferenceSpaceType, Session, SessionState, Space,
    SpaceLocation, Swapchain, SwapchainCreateFlags, SwapchainCreateInfo, SwapchainUsageFlags,
    SystemId, Version, View, ViewConfigurationType, ViewConfigurationView,
};
use openxr_sys::{
    CompositionLayerFlags, Duration as XrDuration, EnvironmentBlendMode, Extent2Di, Offset2Di,
    Rect2Di, Time,
};
use std::ffi::{c_void, CStr};

pub type Backend = OpenGlEs;

pub struct OpenXRComponent<G: Graphics> {
    pub xr_instance: Instance,
    pub xr_session: Session<G>,
    pub frame_waiter: FrameWaiter,
    pub frame_stream: FrameStream<G>,
    pub xr_space: Space,
    pub xr_swapchain_images: Vec<Vec<G::SwapchainImage>>,
    pub xr_swapchains: Vec<Swapchain<G>>,
    pub view_config_views: Vec<ViewConfigurationView>,
}

impl<G: Graphics> Drop for OpenXRComponent<G> {
    fn drop(&mut self) {
        if let Err(e) = self.xr_session.end() {
            self.complain_about_error(e);
        }
    }
}

impl<G: Graphics> OpenXRComponent<G> {
    /// # Safety
    /// the gl_display and gl_context are passed to the OpenXR create_session() call.
    /// How you get them will vary by architecture.
    ///
    /// Android EGL:
    /// ```
    /// # use glutin::display::{AsRawDisplay, Display, DisplayApiPreference, RawDisplay};
    /// let raw_display = event_loop.raw_display_handle();
    ///
    ///  let Display::Egl(glutin_display) =
    ///      unsafe { glutin::display::Display::new(raw_display, DisplayApiPreference::Egl) }?;
    ///
    ///  let RawDisplay::Egl(display_ptr) = glutin_display.raw_display();
    /// ```
    pub fn new(
        entry: &Entry,
        info: &<G as Graphics>::SessionCreateInfo,
        acceptable_format: impl Fn(&G::Format) -> bool,
        pre_session_check: impl Fn(&Instance, SystemId) -> Result<(), XrErrorWrapped>,
    ) -> Result<Self, XrErrorWrapped> {
        let instance = {
            let application_info = ApplicationInfo {
                application_name: "GStreamer OpenXR video sink",
                application_version: 0x1,
                engine_name: "GStreamer",
                engine_version: 0x1110000,
            };
            let mut enabled_extensions = ExtensionSet::default();
            enabled_extensions.khr_opengl_es_enable = true;
            #[cfg(target_os = "android")]
            {
                enabled_extensions.khr_android_create_instance = true;
            }

            let tmp: Result<Instance, openxr_sys::Result> =
                entry.create_instance(&application_info, &enabled_extensions, &[]);
            tmp.annotate_if_err(None, "failed to create XR instance ")?
        };

        let system_id = instance
            .system(FormFactor::HEAD_MOUNTED_DISPLAY)
            .annotate_if_err(Some(&instance), "failed to get system id")?;

        let view_config_views = instance
            .enumerate_view_configuration_views(system_id, ViewConfigurationType::PRIMARY_STEREO)
            .annotate_if_err(Some(&instance), "failed to enumerate configuration views")?;

        pre_session_check(&instance, system_id)?;

        let (xr_session, frame_waiter, frame_stream) = {
            unsafe { instance.create_session::<G>(system_id, info) }
                .annotate_if_err(Some(&instance), "failed to create session")?
        };

        let xr_space = xr_session
            .create_reference_space(
                ReferenceSpaceType::LOCAL,
                Posef {
                    orientation: Quaternionf {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                    position: Default::default(),
                },
            )
            .annotate_if_err(Some(&instance), "failed to create refrence space")?;

        {
            Self::loop_poll_until_ready(&instance)?;
        }

        xr_session
            .begin(ViewConfigurationType::PRIMARY_STEREO)
            .annotate_if_err(Some(&instance), "failed to begin session")?;

        let swapchain_format = {
            let swapchain_formats = xr_session
                .enumerate_swapchain_formats()
                .annotate_if_err(Some(&instance), "failed to enumerate swapchain formats")?;

            let swapchain_format = swapchain_formats.into_iter().find(acceptable_format);

            match swapchain_format {
                None => {
                    return Err(XrErrorWrapped::simple(
                        "failed to find a supported swapchain format",
                    ));
                }
                Some(fmt) => fmt,
            }
        };

        let xr_swapchains = {
            let mut xr_swapchains = vec![];

            for view_config_i in view_config_views.iter() {
                debug!(
                    "view config recommended size {}x{}",
                    view_config_i.recommended_image_rect_width,
                    view_config_i.recommended_image_rect_height
                );
                let swapchain_create_info = SwapchainCreateInfo::<G> {
                    create_flags: SwapchainCreateFlags::EMPTY,
                    usage_flags: SwapchainUsageFlags::SAMPLED
                        | SwapchainUsageFlags::COLOR_ATTACHMENT,
                    format: swapchain_format,
                    sample_count: 1,
                    width: view_config_i.recommended_image_rect_width,
                    height: view_config_i.recommended_image_rect_height,
                    face_count: 1,
                    array_size: 1,
                    mip_count: 1,
                };
                let swapchain = xr_session
                    .create_swapchain(&swapchain_create_info)
                    // .unwrap();
                    .annotate_if_err(Some(&instance), "failed to create swapchain")?;

                xr_swapchains.push(swapchain);
            }

            xr_swapchains
        };

        debug!(
            "fetching swapchain images for {} swapchains",
            xr_swapchains.len()
        );
        let xr_swapchain_images = {
            let mut swapchain_images = vec![];
            for (i, swapchain) in xr_swapchains.iter().enumerate() {
                let images = swapchain
                    .enumerate_images()
                    .annotate_if_err(Some(&instance), "failed to enumerate swapchain images")?;
                debug!("swapchain[{}] has {} images", i, images.len());
                swapchain_images.push(images);
            }

            swapchain_images
        };

        let thing = Self {
            xr_instance: instance,
            xr_session,
            frame_waiter,
            frame_stream,
            xr_space,
            xr_swapchain_images,
            xr_swapchains,
            view_config_views,
        };
        Ok(thing)
    }

    pub fn loop_poll_until_ready(instance: &Instance) -> Result<(), XrErrorWrapped> {
        let mut event_data_buffer2 = Default::default();
        loop {
            match instance.poll_event(&mut event_data_buffer2) {
                Ok(None) => continue,
                Ok(Some(event)) => match event {
                    Event::SessionStateChanged(event) => {
                        if event.state() == SessionState::READY {
                            return Ok(());
                        } else {
                            warn!("unhandled session state event: {:?}", event.state());
                        }
                    }
                    _ => {
                        debug!("ignoring event ");
                    }
                },
                Err(result) => {
                    return Err(XrErrorWrapped::build(
                        result,
                        Some(&instance),
                        "failed to poll for events",
                    ));
                }
            };
        }
    }

    pub fn view_count(&self) -> usize {
        self.view_config_views.len()
    }

    pub fn poll_till_no_events(&mut self) -> Result<LoopStatus, XrResult> {
        let openxr_bits = self;
        let mut event_data_buffer = EventDataBuffer::new();
        loop {
            match openxr_bits.xr_instance.poll_event(&mut event_data_buffer) {
                Ok(Some(evt)) => {
                    if let Event::SessionStateChanged(ch) = evt {
                        if let SessionState::STOPPING = ch.state() {
                            return Ok(LoopStatus::PleaseStop);
                        }
                    }
                    info!(
                        "ignoring event ",
                        //event_data_buffer.ty.into_raw()
                    );
                }
                Ok(None) => return Ok(LoopStatus::Groovy), // EVENT_UNAVAILALBE,
                Err(result) => return Err(result),
            };
        }
    }

    /// Get the frame state and provide it to the `before_paint` closure to
    /// calculate app-specific data.
    /// Then use the `paint_one_view` closure with that app-specific data to
    /// render all the camera views needed by the openxr system
    pub fn paint_vr_multiview<T>(
        &mut self,
        before_paint: impl FnOnce(&Self, &FrameState) -> T,
        mut paint_one_view: impl FnMut(&View, &ViewConfigurationView, Time, &G::SwapchainImage, &mut T),
        mut after_paint: impl FnMut(&Self, &FrameState, T),
        view_configuration_type: ViewConfigurationType,
    ) -> Result<(), XrErrorWrapped> {
        let frame_state = self
            .frame_waiter
            .wait()
            .annotate_if_err(None, "failed to wait for frame")?;
        let predicted_display_time: Time = frame_state.predicted_display_time;

        self.frame_stream
            .begin()
            .annotate_if_err(None, "failed to frame_stream.begin")?;

        let (_flags, views) = self
            .xr_session
            .locate_views(
                view_configuration_type,
                predicted_display_time,
                &self.xr_space,
            )
            .annotate_if_err(None, "failed to locate_views")?;

        let mut malfunctions = vec![];

        let mut arg = before_paint(self, &frame_state);

        for (swapchain, sci, view_i, vcv) in izip!(
            self.xr_swapchains.iter_mut(),
            &self.xr_swapchain_images,
            views.iter(),
            self.view_config_views.iter(),
        ) {
            let buffer_index = match swapchain.acquire_image() {
                Ok(x) => x,
                Err(result) => {
                    malfunctions.push(XrErrorWrapped::build(
                        result,
                        None,
                        "failed to acquire swapchain image",
                    ));
                    continue;
                }
            };

            if let Err(result) = swapchain.wait_image(XrDuration::INFINITE) {
                malfunctions.push(XrErrorWrapped::build(
                    result,
                    None,
                    "failed to wait for swapchain image",
                ));
                continue;
            };

            let color_buffer = &sci[buffer_index as usize];

            paint_one_view(view_i, vcv, predicted_display_time, color_buffer, &mut arg);

            if let Err(result) = swapchain.release_image() {
                malfunctions.push(XrErrorWrapped::build(
                    result,
                    None,
                    "failed to release swapchain image",
                ));
                continue;
            }
        }

        after_paint(self, &frame_state, arg);

        for err in &malfunctions {
            log::error!("malfunction while painting OpenXR views {}", err);
        }
        if let Some(err) = malfunctions.into_iter().next() {
            (Err(err))?;
        }

        let projection_views: Vec<_> = {
            izip!(
                views.iter(),
                self.xr_swapchains.iter(),
                self.view_config_views.iter()
            )
            .map(|(view, swapchain, view_config_view)| {
                projection_view_for(view, swapchain, view_config_view)
            })
            .collect()
        };

        {
            let projection_layer = CompositionLayerProjection::new()
                .layer_flags(CompositionLayerFlags::EMPTY)
                .space(&self.xr_space)
                .views(projection_views.as_slice());

            let projection_layers: Vec<&CompositionLayerBase<G>> = vec![&projection_layer];

            self.frame_stream
                .end(
                    predicted_display_time,
                    EnvironmentBlendMode::OPAQUE,
                    projection_layers.as_slice(),
                )
                .annotate_if_err(None, "failed to frame_stream.end")?;
        }

        Ok(())
    }

    pub fn complain_about_error(&self, result: XrResult) {
        Self::complain_about_error0(&self.xr_instance.as_raw(), result)
    }

    pub fn complain_about_error0(instance: &openxr_sys::Instance, result: XrResult) {
        error!("{}", message_for_error(instance, result));
    }
}

#[cfg(target_os = "android")]
impl OpenXRComponent<OpenGlEs> {
    /// # Safety
    /// the gl_display and gl_context are passed to the OpenXR create_session() call.
    /// How you get them will vary by architecture.
    ///
    /// Android EGL:
    /// ```
    /// # use glutin::display::{AsRawDisplay, Display, DisplayApiPreference, RawDisplay};
    /// let raw_display = event_loop.raw_display_handle();
    ///
    ///  let Display::Egl(glutin_display) =
    ///      unsafe { glutin::display::Display::new(raw_display, DisplayApiPreference::Egl) }?;
    ///
    ///  let RawDisplay::Egl(display_ptr) = glutin_display.raw_display();
    /// ```
    pub fn new_android(
        gl_display: *mut c_void,
        gl_context: *mut c_void,
    ) -> Result<Self, XrErrorWrapped> {
        let entry: Entry = Entry::linked();
        {
            if let Err(e) = entry.initialize_android_loader() {
                return Err(XrErrorWrapped::simple(format!(
                    "failed to initialize android loader  : {}",
                    e
                )));
            }
        }

        let mut gl_major_version = -1;
        let mut gl_minor_version = -1;
        unsafe { gl::GetIntegerv(gl::MAJOR_VERSION, &mut gl_major_version) };
        unsafe { gl::GetIntegerv(gl::MINOR_VERSION, &mut gl_minor_version) };
        let session_pre_check =
            |instance: &Instance, system_id: SystemId| -> Result<(), XrErrorWrapped> {
                debug!("time to check the version requirements");

                check_version_requirements(instance, system_id, gl_major_version, gl_minor_version)
            };

        let info = openxr::opengles::SessionCreateInfo::Android {
            context: gl_context,
            display: gl_display,
            //system_id,
            config: std::ptr::null_mut(),
        };

        let acceptable_format = |&fmt: &u32| {
            fmt == gl::RGBA8
                || fmt == gl::RGBA8_SNORM
                || (fmt == gl::SRGB8_ALPHA8 && gl_major_version >= 3)
        };

        Self::new(&entry, &info, acceptable_format, session_pre_check)
    }
}

pub fn message_for_error(instance: &openxr_sys::Instance, result: XrResult) -> String {
    let mut msg = [0; MAX_RESULT_STRING_SIZE];
    if XrResult::SUCCESS.into_raw()
        != unsafe {
            let msg_ptr = &mut msg as *mut u8;
            result_to_string(*instance, result, msg_ptr as *mut _).into_raw()
        }
    {
        msg[0] = 0;
    }
    match CStr::from_bytes_until_nul(&msg) {
        Ok(msg) => {
            format!("OpenXR call failed: {:?} ({})", msg, result)
        }
        Err(_) => {
            format!("OpenXR call failed: {:x?} ({})", msg, result)
        }
    }
}

pub fn check_version_requirements(
    instance: &Instance,
    system_id: SystemId,
    gl_major_version: GLint,
    gl_minor_version: GLint,
) -> Result<(), XrErrorWrapped> {
    let tmp: Result<_, openxr_sys::Result> = Backend::requirements(instance, system_id);
    let graphics_requirements =
        tmp.annotate_if_err(Some(instance), "failed to get requirements")?;

    let gl_version = Version::new(gl_major_version as u16, gl_minor_version as u16, 0);
    if graphics_requirements.min_api_version_supported > gl_version {
        return Err(XrErrorWrapped::simple(format!(
            "OpenXR runtime doesn't support the OpenGL version {} > {}",
            graphics_requirements.min_api_version_supported, gl_version
        )));
    }
    Ok(())
}

pub fn projection_view_for<'a, G: Graphics>(
    view: &View,
    swapchain: &'a Swapchain<G>,
    view_config_view: &ViewConfigurationView,
) -> openxr::CompositionLayerProjectionView<'a, G> {
    openxr::CompositionLayerProjectionView::new()
        .pose(view.pose)
        .fov(view.fov)
        .sub_image(
            openxr::SwapchainSubImage::<G>::new()
                .swapchain(swapchain)
                .image_rect(Rect2Di {
                    offset: Offset2Di { x: 0, y: 0 },
                    extent: Extent2Di {
                        width: view_config_view.recommended_image_rect_width as i32,
                        height: view_config_view.recommended_image_rect_height as i32,
                    },
                })
                .image_array_index(0),
        )
}

//

pub struct RightHandTracker {
    pub space: Space,
}

impl RightHandTracker {
    pub fn new<G: Graphics>(
        instance: &Instance,
        xr_session: &Session<G>,
        action_set: &ActionSet,
    ) -> Result<Self, XrErrorWrapped> {
        let user_hand_left = instance
            .string_to_path("/user/hand/left")
            .annotate_if_err(Some(instance), "failed to ")?;
        let user_hand_right = instance
            .string_to_path("/user/hand/right")
            .annotate_if_err(Some(instance), "failed to ")?;
        let pose_action = action_set
            .create_action::<Posef>(
                "hand_pose",
                "controller 1",
                &[user_hand_left, user_hand_right],
            )
            .annotate_if_err(Some(instance), "failed to ")?;
        let left_grip_pose = instance
            .string_to_path("/user/hand/left/input/grip/pose")
            .annotate_if_err(Some(instance), "failed to ")?;
        let right_grip_pose = instance
            .string_to_path("/user/hand/right/input/grip/pose")
            .annotate_if_err(Some(instance), "failed to ")?;
        let bindings = [
            Binding::new(&pose_action, left_grip_pose),
            Binding::new(&pose_action, right_grip_pose),
        ];
        {
            let interaction_profile = instance
                .string_to_path("/interaction_profiles/khr/simple_controller")
                .annotate_if_err(Some(instance), "failed to ")?;

            instance
                .suggest_interaction_profile_bindings(interaction_profile, &bindings)
                .annotate_if_err(Some(instance), "failed to ")?;
        }

        {
            let interaction_profile = instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .annotate_if_err(Some(instance), "failed to ")?;
            instance
                .suggest_interaction_profile_bindings(interaction_profile, &bindings)
                .annotate_if_err(Some(instance), "failed to ")?;
        }

        let mut posef = Posef::default();
        posef.orientation.w = 1.0;
        let space = pose_action
            .create_space(xr_session.clone(), user_hand_right, posef)
            .annotate_if_err(Some(instance), "failed to ")?;

        Ok(Self { space })
    }

    pub fn action_set_from<G: Graphics>(
        instance: &Instance,
        xr_session: &Session<G>,
    ) -> Result<(ActionSet, Self), XrErrorWrapped> {
        let action_set = instance
            .create_action_set("pants", "pants", 0)
            .annotate_if_err(Some(instance), "failed to create_action_set")?;

        let right_hand_tracker = Self::new(instance, xr_session, &action_set)?;

        xr_session
            .attach_action_sets(&[&action_set])
            .annotate_if_err(Some(instance), "failed to attach_action_sets")?;

        Ok((action_set, right_hand_tracker))
    }

    pub fn locate(&self, base: &Space, time: Time) -> Result<SpaceLocation, XrResult> {
        self.space.locate(base, time)
    }
}

//

/// the return value for our canned event processing loop
#[derive(PartialEq, Eq)]
pub enum LoopStatus {
    /// the XR state changed to STOPPING
    PleaseStop,
    /// Nothing weird happened, carry on
    Groovy,
}
