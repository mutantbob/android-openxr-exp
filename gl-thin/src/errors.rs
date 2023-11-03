use std::fmt::{Debug, Display, Formatter};

pub struct XrErrorWrapped {
    pub xr_err: Option<String>,
    pub detail: String,
}

impl XrErrorWrapped {
    pub fn new(xr_err: String, detail: impl Into<String>) -> Self {
        Self {
            xr_err: Some(xr_err),
            detail: detail.into(),
        }
    }
    pub fn simple(detail: impl Into<String>) -> Self {
        Self {
            xr_err: None,
            detail: detail.into(),
        }
    }

    #[cfg(feature = "openxr")]
    pub fn build(
        e: openxr_sys::Result,
        instance: Option<&openxr::Instance>,
        msg: impl Into<String>,
    ) -> XrErrorWrapped {
        let x = match instance {
            Some(instance) => crate::openxr_helpers::message_for_error(&instance.as_raw(), e),
            None => format!("OpenXR failed {:?}", e),
        };
        XrErrorWrapped::new(x, msg.into())
    }
}

impl Debug for XrErrorWrapped {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = self.xr_err.as_ref() {
            write!(f, "{}: {}", self.detail, msg)
        } else {
            f.write_str(&self.detail)
        }
    }
}

impl Display for XrErrorWrapped {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl std::error::Error for XrErrorWrapped {}

//

#[cfg(feature = "openxr")]
/// This only exists so I can chain a call onto a Result to convert it
pub trait Wrappable<T> {
    fn annotate_if_err<S: Into<String>>(
        self,
        instance: Option<&openxr::Instance>,
        msg: S,
    ) -> Result<T, XrErrorWrapped>;
}

#[cfg(feature = "openxr")]
impl<T> Wrappable<T> for Result<T, openxr_sys::Result> {
    fn annotate_if_err<S: Into<String>>(
        self,
        instance: Option<&openxr::Instance>,
        msg: S,
    ) -> Result<T, XrErrorWrapped> {
        self.map_err(|e| XrErrorWrapped::build(e, instance, msg))
    }
}
