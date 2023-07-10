use std::fmt::Write;

pub fn main() {
    let openxr_libdir = match std::env::var("OPENXR_LIBDIR") {
        Ok(dir) => dir,
        Err(_) => {
            if false {
                dump_env_variables();
            }
            const EXAMPLE: &str =
                "ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/libopenxr_loader.so?";
            panic!(
                "missing OPENXR_LIBDIR environment variable, what directory contains {}?",
                EXAMPLE
            )
        }
    };
    println!("cargo:rustc-link-search={}", openxr_libdir);

    //

    let android_libdir = match std::env::var("ANDROID_GST_LIBDIR") {
        Ok(dir) => dir,
        Err(_) => {
            const EXAMPLE: &str = "gst-android/arm64/lib/libgstbase*.a";
            panic!(
                "missing ANDROID_GST_LIBDIR environment variable, what directory contains {}?",
                EXAMPLE
            )
        }
    };

    println!("cargo:rustc-link-search={}", android_libdir);
    println!("cargo:rustc-link-lib=ffi");
}

fn dump_env_variables() {
    // let mut buffer = File::create("/tmp/build.txt").unwrap();
    let mut buffer = String::new();
    for (key, val) in std::env::vars() {
        if val.contains("64") {
            writeln!(&mut buffer, " key {}={}", key, val).unwrap();
        }
    }

    panic!("{}", buffer);
}
