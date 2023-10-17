use crate::rainbow_triangle::{RainbowTriangle, Suzanne, TextMessage};
use gl_thin::gl_fancy::GPUState;
use gl_thin::gl_helper::{explode_if_gl_error, GLErrorWrapper};
use gl_thin::linear::{
    xr_matrix4x4f_create_from_quaternion, xr_matrix4x4f_create_projection_fov,
    xr_matrix4x4f_create_scale, xr_matrix4x4f_create_translation,
    xr_matrix4x4f_create_translation_rotation_scale, xr_matrix4x4f_create_translation_v,
    xr_matrix4x4f_invert_rigid_body, GraphicsAPI, XrFovf, XrMatrix4x4f, XrQuaternionf, XrVector3f,
};
use openxr::SpaceLocation;
use openxr_sys::Time;
use std::f32::consts::{PI, TAU};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MyScene {
    pub rainbow_triangle: RainbowTriangle<'static>,
    pub suzanne: Suzanne,
    pub text_message: TextMessage,
}

impl MyScene {
    pub fn new(gpu_state: &mut GPUState) -> Result<Self, GLErrorWrapper> {
        Ok(MyScene {
            rainbow_triangle: RainbowTriangle::new(gpu_state)?,
            suzanne: Suzanne::new(gpu_state)?,
            text_message: TextMessage::new(gpu_state)?,
        })
    }

    pub fn draw(
        &self,
        fov: &XrFovf,
        rotation: &XrQuaternionf,
        translation: &XrVector3f,
        _time: Time,
        gpu_state: &mut GPUState,
        controller_1: &Option<SpaceLocation>,
    ) -> Result<(), GLErrorWrapper> {
        let (theta, rotation_matrix) = rotation_matrix_for_now();

        unsafe {
            let green = (theta.sin() + 1.0) * 0.5;
            gl::ClearColor(0.0, green, 0.3, 1.0)
        };
        explode_if_gl_error()?;
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT) };
        explode_if_gl_error()?;

        unsafe { gl::Enable(gl::DEPTH_TEST) };
        explode_if_gl_error()?;

        if true {
            unsafe {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
        }

        //

        let matrix_pv = {
            let projection_matrix = xr_matrix4x4f_create_projection_fov(
                GraphicsAPI::GraphicsOpenGL,
                fov,
                0.01,
                10_000.0,
            );
            //log::debug!("matrix = {}", debug_string_matrix(&projection_matrix),);
            let view_matrix = xr_matrix4x4f_create_translation_rotation_scale(
                translation,
                rotation,
                &XrVector3f::default_scale(),
            );
            let inverse_view_matrix = xr_matrix4x4f_invert_rigid_body(&view_matrix);

            projection_matrix * inverse_view_matrix
        };

        {
            let model = xr_matrix4x4f_create_translation(1.0, 0.0, -2.0);
            let model = model * rotation_matrix;
            self.rainbow_triangle
                .paint_color_triangle(&(matrix_pv * model), gpu_state)?;
        }

        if let Some(controller_1) = controller_1 {
            let model = {
                let translate =
                    xr_matrix4x4f_create_translation_v(&controller_1.pose.position.into());
                let upright = matrix_rotation_about_x(PI);
                let rotation_matrix =
                    xr_matrix4x4f_create_from_quaternion(&controller_1.pose.orientation.into());
                let scale1 = 0.05;
                let scale = xr_matrix4x4f_create_scale(scale1, scale1, scale1);
                let model = scale;
                let model = upright * model;
                let model = rotation_matrix * model;
                translate * model
            };
            self.suzanne.draw(
                &model,
                &matrix_pv,
                &[0.0, 1.0, 0.0],
                &[0.0, 0.0, 1.0],
                self.suzanne.index_count(),
                gpu_state,
            )?;
        }

        {
            let model = {
                let translate = xr_matrix4x4f_create_translation(0.0, -0.5, -3.0);
                let s = 0.2;
                let scale = xr_matrix4x4f_create_scale(s, s, s);
                let model = scale;
                // let model = upright*model;
                // let model = rotation_matrix*model;
                translate * model
            };
            let matrix = matrix_pv * model;
            self.text_message
                .draw(&matrix, self.text_message.index_count(), gpu_state)
        }
    }
}

fn rotation_matrix_for_now() -> (f32, XrMatrix4x4f) {
    let theta = if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let tm = duration.as_millis();
        let phase = tm % 5000;
        TAU * phase as f32 / 5000.0
    } else {
        0.0
    };
    let rotation_matrix = if true {
        matrix_rotation_about_y(theta)
    } else {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0f32,
        ]
        .into()
    };
    (theta, rotation_matrix)
}

#[rustfmt::skip]
pub fn matrix_rotation_about_z(theta: f32) -> XrMatrix4x4f {
    [
        theta.cos(), theta.sin(), 0.0, 0.0, //
        -theta.sin(), theta.cos(), 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0f32,
    ].into()
}

#[rustfmt::skip]
pub fn matrix_rotation_about_y(theta: f32) -> XrMatrix4x4f {
    [
        theta.cos(), 0.0, theta.sin(), 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        -theta.sin(), 0.0, theta.cos(), 0.0, //
        0.0, 0.0, 0.0, 1.0f32,
    ].into()
}

#[rustfmt::skip]
pub fn matrix_rotation_about_x(theta: f32) -> XrMatrix4x4f {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, theta.cos(), theta.sin(), 0.0,
        0.0, -theta.sin(), theta.cos(), 0.0,
        0.0, 0.0, 0.0, 1.0f32,
    ].into()
}
