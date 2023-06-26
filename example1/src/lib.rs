// This code is loosely based on
// glium's examples/triangle.rs
// and
// android-activity's examples-na-winit-glutin

use crate::gl_helper::initialize_gl_using_egli;
use android_activity::AndroidApp;
use drawcore::ActiveRenderer;
use std::ops::Add;
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget};
use winit::platform::android::EventLoopBuilderExtAndroid;

pub mod drawcore;
pub mod errors;
pub mod flat_color_shader;
pub mod gl_fancy;
pub mod gl_helper;
pub mod kludges;
pub mod linear;
pub mod openxr_helpers;
pub mod rainbow_triangle;
pub mod sun_phong_shader;
pub mod suzanne;

//

pub trait Scene {
    fn draw(&mut self);
}

pub enum AppState<T: Scene> {
    Paused,
    Active(T),
}

impl<T: Scene> Default for AppState<T> {
    fn default() -> Self {
        Self::Paused
    }
}

//

fn event_loop_one_pass<T: Scene, X: std::fmt::Debug, E: std::fmt::Debug>(
    event: Event<X>,
    event_loop: &EventLoopWindowTarget<X>,
    control_flow: &mut ControlFlow,
    app: &mut AppState<T>,
    factory: impl Fn(&EventLoopWindowTarget<X>) -> Result<T, E>,
) {
    log::trace!("Received Winit event: {event:?}");

    let static_graphics = false;

    *control_flow = match app {
        AppState::Paused => ControlFlow::Wait,
        AppState::Active(_) => {
            if static_graphics {
                ControlFlow::Poll
            } else {
                // trigger redraws every 6 milliseconds
                ControlFlow::WaitUntil(Instant::now().add(Duration::from_millis(6)))
            }
        }
    };

    match event {
        Event::Resumed => {
            log::debug!("resume");
            *app = AppState::Active(factory(event_loop).unwrap());
        }
        Event::Suspended => {
            log::debug!("suspend");
            // log::trace!("Suspended, dropping surface state...");
            // app.surface_state = None;
            *app = AppState::Paused;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_size),
            ..
        } => {
            // Winit: doesn't currently implicitly request a redraw
            // for a resize which may be required on some platforms...
            if let AppState::Active(_) = app {
                *control_flow = ControlFlow::Poll; // this should trigger a redraw via NewEvents
            }
        }
        Event::RedrawRequested(_) => {
            log::trace!("Handling Redraw Request");
            if let AppState::Active(app) = app {
                app.draw();
            }
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::NewEvents(_) => {
            if let AppState::Active(app) = app {
                app.draw();
            }
        }
        _ => {}
    }
}

//#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );

    // use glium::glutin::platform::android::EventLoopBuilderExtAndroid;

    log::debug!("bob test");

    // initialize_gl_using_egli();

    let mut builder: //winit::event_loop::
        EventLoopBuilder<_> = EventLoopBuilder::new();
    let event_loop: EventLoop<()> = builder.with_android_app(android_app).build();

    log::debug!("got event loop");

    let mut app = AppState::<ActiveRenderer>::default();
    event_loop.run(move |evt, e_loop, ctx| {
        event_loop_one_pass(evt, e_loop, ctx, &mut app, |event_loop| {
            initialize_gl_using_egli();

            ActiveRenderer::new(event_loop)
        })
    });
}