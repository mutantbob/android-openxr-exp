// these are as-needes copies of inline functions from the OpenXR-SDK xr_linear.h

use openxr_sys::{Fovf, Quaternionf, Vector3f};

#[derive(Copy, Clone, Debug)]
pub struct XrMatrix4x4f {
    m: [f32; 16],
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
#[repr(C)]
pub enum GraphicsAPI {
    GraphicsVulkan = 0,
    GraphicsOpenGL = 1,
    GraphicsOpenGLES = 2,
    GraphicsD2D = 3,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct XrFovf {
    pub angle_left: f32,
    pub angle_right: f32,
    pub angle_up: f32,
    pub angle_down: f32,
}

impl From<Fovf> for XrFovf {
    fn from(value: Fovf) -> Self {
        XrFovf {
            angle_left: value.angle_left,
            angle_right: value.angle_right,
            angle_up: value.angle_up,
            angle_down: value.angle_down,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[repr(C)]
pub struct XrVector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl XrVector3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn default_translation() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
    pub fn default_scale() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    pub fn scale(scale: f32) -> XrVector3f {
        Self {
            x: scale,
            y: scale,
            z: scale,
        }
    }
}

impl From<&XrVector3f> for Vector3f {
    fn from(val: &XrVector3f) -> Self {
        Vector3f {
            x: val.x,
            y: val.y,
            z: val.z,
        }
    }
}

impl From<Vector3f> for XrVector3f {
    fn from(value: Vector3f) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl std::ops::Add for XrVector3f {
    type Output = XrVector3f;

    fn add(self, rhs: Self) -> Self::Output {
        XrVector3f {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub for XrVector3f {
    type Output = XrVector3f;

    fn sub(self, rhs: Self) -> Self::Output {
        XrVector3f {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Neg for XrVector3f {
    type Output = XrVector3f;

    fn neg(self) -> Self::Output {
        XrVector3f {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

//

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct XrQuaternionf {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl XrQuaternionf {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl Default for XrQuaternionf {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl From<Quaternionf> for XrQuaternionf {
    fn from(value: Quaternionf) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

impl std::ops::Mul for XrQuaternionf {
    type Output = XrQuaternionf;

    fn mul(self, q2: Self) -> Self::Output {
        let q1 = self;
        let x = q1.x * q2.w + q1.y * q2.z - q1.z * q2.y + q1.w * q2.x;
        let y = -q1.x * q2.z + q1.y * q2.w + q1.z * q2.x + q1.w * q2.y;
        let z = q1.x * q2.y - q1.y * q2.x + q1.z * q2.w + q1.w * q2.z;
        let w = -q1.x * q2.x - q1.y * q2.y - q1.z * q2.z + q1.w * q2.w;

        Self::Output::new(x, y, z, w)
    }
}

//

impl XrMatrix4x4f {
    pub fn new(m: [f32; 16]) -> Self {
        XrMatrix4x4f { m }
    }

    pub fn slice(&self) -> &[f32; 16] {
        &self.m
    }
}

impl From<[f32; 16]> for XrMatrix4x4f {
    fn from(value: [f32; 16]) -> Self {
        XrMatrix4x4f::new(value)
    }
}

/*
#[rustfmt::skip]
impl_op_ex!(* |a: &XrMatrix4x4f, b: &XrMatrix4x4f| -> XrMatrix4x4f { xr_matrix4x4f_multiply(a, b) });*/
impl<'a, 'b> std::ops::Mul<&'a XrMatrix4x4f> for &'b XrMatrix4x4f {
    type Output = XrMatrix4x4f;

    fn mul(self, rhs: &XrMatrix4x4f) -> Self::Output {
        xr_matrix4x4f_multiply(self, rhs)
    }
}
impl<'a> std::ops::Mul<XrMatrix4x4f> for &'a XrMatrix4x4f {
    type Output = XrMatrix4x4f;

    fn mul(self, rhs: XrMatrix4x4f) -> Self::Output {
        xr_matrix4x4f_multiply(self, &rhs)
    }
}
impl<'a> std::ops::Mul<&'a XrMatrix4x4f> for XrMatrix4x4f {
    type Output = XrMatrix4x4f;

    fn mul(self, rhs: &XrMatrix4x4f) -> Self::Output {
        xr_matrix4x4f_multiply(&self, rhs)
    }
}
impl std::ops::Mul<XrMatrix4x4f> for XrMatrix4x4f {
    type Output = XrMatrix4x4f;

    fn mul(self, rhs: XrMatrix4x4f) -> Self::Output {
        xr_matrix4x4f_multiply(&self, &rhs)
    }
}

#[rustfmt::skip]
pub fn xr_matrix4x4f_identity() -> XrMatrix4x4f {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ].into()
}

pub fn xr_matrix4x4f_create_projection_fov(
    graphics_api: GraphicsAPI,
    fov: &XrFovf,
    near_z: f32,
    far_z: f32,
) -> XrMatrix4x4f {
    let tan_left = fov.angle_left.tan();
    let tan_right = fov.angle_right.tan();

    let tan_down = fov.angle_down.tan();
    let tan_up = fov.angle_up.tan();

    xr_matrix4x4f_create_projection(
        graphics_api,
        tan_left,
        tan_right,
        tan_up,
        tan_down,
        near_z,
        far_z,
    )
}

pub fn xr_matrix4x4f_create_projection(
    graphics_api: GraphicsAPI,
    tan_angle_left: f32,
    tan_angle_right: f32,
    tan_angle_up: f32,
    tan_angle_down: f32,
    near_z: f32,
    far_z: f32,
) -> XrMatrix4x4f {
    let tan_angle_width = tan_angle_right - tan_angle_left;

    // Set to tan_angle_down - tan_angle_up for a clip space with positive Y down (Vulkan).
    // Set to tan_angle_up - tan_angle_down for a clip space with positive Y up (OpenGL / D3D / Metal).
    let tan_angle_height = if graphics_api == GraphicsAPI::GraphicsVulkan {
        tan_angle_down - tan_angle_up
    } else {
        tan_angle_up - tan_angle_down
    };

    // Set to near_z for a [-1,1] Z clip space (OpenGL / OpenGL ES).
    // Set to zero for a [0,1] Z clip space (Vulkan / D3D / Metal).
    let offset_z = if graphics_api == GraphicsAPI::GraphicsOpenGL
        || graphics_api == GraphicsAPI::GraphicsOpenGLES
    {
        near_z
    } else {
        0.0
    };

    if far_z <= near_z {
        // place the far plane at infinity
        let m0 = 2.0 / tan_angle_width;
        let m4 = 0.0;
        let m8 = (tan_angle_right + tan_angle_left) / tan_angle_width;
        let m12 = 0.0;

        let m1 = 0.0;
        let m5 = 2.0 / tan_angle_height;
        let m9 = (tan_angle_up + tan_angle_down) / tan_angle_height;
        let m13 = 0.0;

        let m2 = 0.0;
        let m6 = 0.0;
        let m10 = -1.0;
        let m14 = -(near_z + offset_z);

        let m3 = 0.0;
        let m7 = 0.0;
        let m11 = -1.0;
        let m15 = 0.0;
        [
            m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
        ]
        .into()
    } else {
        // normal projection
        let m0 = 2.0 / tan_angle_width;
        let m4 = 0.0;
        let m8 = (tan_angle_right + tan_angle_left) / tan_angle_width;
        let m12 = 0.0;

        let m1 = 0.0;
        let m5 = 2.0 / tan_angle_height;
        let m9 = (tan_angle_up + tan_angle_down) / tan_angle_height;
        let m13 = 0.0;

        let m2 = 0.0;
        let m6 = 0.0;
        let m10 = -(far_z + offset_z) / (far_z - near_z);
        let m14 = -(far_z * (near_z + offset_z)) / (far_z - near_z);

        let m3 = 0.0;
        let m7 = 0.0;
        let m11 = -1.0;
        let m15 = 0.0;
        [
            m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
        ]
        .into()
    }
}

pub fn xr_matrix4x4f_create_translation_rotation_scale(
    translation: &XrVector3f,
    rotation: &XrQuaternionf,
    scale: &XrVector3f,
) -> XrMatrix4x4f {
    let scale_matrix = xr_matrix4x4f_create_scale(scale.x, scale.y, scale.z);

    let rotation_matrix = xr_matrix4x4f_create_from_quaternion(rotation);

    let translation_matrix =
        xr_matrix4x4f_create_translation(translation.x, translation.y, translation.z);

    translation_matrix * rotation_matrix * scale_matrix
}

pub fn xr_matrix4x4f_create_translation(dx: f32, dy: f32, dz: f32) -> XrMatrix4x4f {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, dx, dy, dz, 1.0,
    ]
    .into()
}

pub fn xr_matrix4x4f_create_translation_v(xyz: &Vector3f) -> XrMatrix4x4f {
    xr_matrix4x4f_create_translation(xyz.x, xyz.y, xyz.z)
}

pub fn xr_matrix4x4f_create_from_quaternion(quat: &XrQuaternionf) -> XrMatrix4x4f {
    let x2 = quat.x + quat.x;
    let y2 = quat.y + quat.y;
    let z2 = quat.z + quat.z;

    let xx2 = quat.x * x2;
    let yy2 = quat.y * y2;
    let zz2 = quat.z * z2;

    let yz2 = quat.y * z2;
    let wx2 = quat.w * x2;
    let xy2 = quat.x * y2;
    let wz2 = quat.w * z2;
    let xz2 = quat.x * z2;
    let wy2 = quat.w * y2;

    let m0 = 1.0 - yy2 - zz2;
    let m1 = xy2 + wz2;
    let m2 = xz2 - wy2;
    let m3 = 0.0;

    let m4 = xy2 - wz2;
    let m5 = 1.0 - xx2 - zz2;
    let m6 = yz2 + wx2;
    let m7 = 0.0;

    let m8 = xz2 + wy2;
    let m9 = yz2 - wx2;
    let m10 = 1.0 - xx2 - yy2;
    let m11 = 0.0;

    let m12 = 0.0;
    let m13 = 0.0;
    let m14 = 0.0;
    let m15 = 1.0;
    [
        m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
    ]
    .into()
}

pub fn xr_matrix4x4f_create_scale(x: f32, y: f32, z: f32) -> XrMatrix4x4f {
    [
        x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
    .into()
}

pub fn xr_matrix4x4f_multiply(a: &XrMatrix4x4f, b: &XrMatrix4x4f) -> XrMatrix4x4f {
    let m0 = a.m[0] * b.m[0] + a.m[4] * b.m[1] + a.m[8] * b.m[2] + a.m[12] * b.m[3];
    let m1 = a.m[1] * b.m[0] + a.m[5] * b.m[1] + a.m[9] * b.m[2] + a.m[13] * b.m[3];
    let m2 = a.m[2] * b.m[0] + a.m[6] * b.m[1] + a.m[10] * b.m[2] + a.m[14] * b.m[3];
    let m3 = a.m[3] * b.m[0] + a.m[7] * b.m[1] + a.m[11] * b.m[2] + a.m[15] * b.m[3];

    let m4 = a.m[0] * b.m[4] + a.m[4] * b.m[5] + a.m[8] * b.m[6] + a.m[12] * b.m[7];
    let m5 = a.m[1] * b.m[4] + a.m[5] * b.m[5] + a.m[9] * b.m[6] + a.m[13] * b.m[7];
    let m6 = a.m[2] * b.m[4] + a.m[6] * b.m[5] + a.m[10] * b.m[6] + a.m[14] * b.m[7];
    let m7 = a.m[3] * b.m[4] + a.m[7] * b.m[5] + a.m[11] * b.m[6] + a.m[15] * b.m[7];

    let m8 = a.m[0] * b.m[8] + a.m[4] * b.m[9] + a.m[8] * b.m[10] + a.m[12] * b.m[11];
    let m9 = a.m[1] * b.m[8] + a.m[5] * b.m[9] + a.m[9] * b.m[10] + a.m[13] * b.m[11];
    let m10 = a.m[2] * b.m[8] + a.m[6] * b.m[9] + a.m[10] * b.m[10] + a.m[14] * b.m[11];
    let m11 = a.m[3] * b.m[8] + a.m[7] * b.m[9] + a.m[11] * b.m[10] + a.m[15] * b.m[11];

    let m12 = a.m[0] * b.m[12] + a.m[4] * b.m[13] + a.m[8] * b.m[14] + a.m[12] * b.m[15];
    let m13 = a.m[1] * b.m[12] + a.m[5] * b.m[13] + a.m[9] * b.m[14] + a.m[13] * b.m[15];
    let m14 = a.m[2] * b.m[12] + a.m[6] * b.m[13] + a.m[10] * b.m[14] + a.m[14] * b.m[15];
    let m15 = a.m[3] * b.m[12] + a.m[7] * b.m[13] + a.m[11] * b.m[14] + a.m[15] * b.m[15];
    [
        m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
    ]
    .into()
}

pub fn xr_matrix4x4f_invert_rigid_body(src: &XrMatrix4x4f) -> XrMatrix4x4f {
    let m0 = src.m[0];
    let m1 = src.m[4];
    let m2 = src.m[8];
    let m3 = 0.0;
    let m4 = src.m[1];
    let m5 = src.m[5];
    let m6 = src.m[9];
    let m7 = 0.0;
    let m8 = src.m[2];
    let m9 = src.m[6];
    let m10 = src.m[10];
    let m11 = 0.0;
    let m12 = -(src.m[0] * src.m[12] + src.m[1] * src.m[13] + src.m[2] * src.m[14]);
    let m13 = -(src.m[4] * src.m[12] + src.m[5] * src.m[13] + src.m[6] * src.m[14]);
    let m14 = -(src.m[8] * src.m[12] + src.m[9] * src.m[13] + src.m[10] * src.m[14]);
    let m15 = 1.0;
    [
        m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
    ]
    .into()
}

pub fn xr_matrix4x4f_transform_vector3f(m: &XrMatrix4x4f, v: &XrVector3f) -> XrVector3f {
    let w = m.m[3] * v.x + m.m[7] * v.y + m.m[11] * v.z + m.m[15];
    if false {
        log::debug!(
            "w = {} = {}*{} + {}*{} + {}*{} + {}",
            w,
            m.m[3],
            v.x,
            m.m[7],
            v.y,
            m.m[11],
            v.z,
            m.m[15]
        );
    }
    let rcp_w = 1.0 / w;
    let x = (m.m[0] * v.x + m.m[4] * v.y + m.m[8] * v.z + m.m[12]) * rcp_w;
    let y = (m.m[1] * v.x + m.m[5] * v.y + m.m[9] * v.z + m.m[13]) * rcp_w;
    let z = (m.m[2] * v.x + m.m[6] * v.y + m.m[10] * v.z + m.m[14]) * rcp_w;
    XrVector3f { x, y, z }
}
