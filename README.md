# prerequisites
`cargo install cargo-apk`

`cargo install cargo-ndk`

https://developer.oculus.com/downloads/package/oculus-openxr-mobile-sdk/

```
git clone https://github.com/GStreamer/cerbero
```

# building


```
ANDROID_HOME=~/Android/Sdk/ \
 ANDROID_NDK_ROOT=~/Android/Sdk/ndk/25.2.9519653/ \
 OPENXR_LIBDIR=~/vendor/ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/ \
 ANDROID_GST_LIBDIR=~/vendor/gst-android/arm64/lib/ \
    cargo apk run

ANDROID_HOME=~/Android/Sdk/ \
 ANDROID_NDK_ROOT=~/Android/Sdk/ndk/25.2.9519653/ \
 OPENXR_LIBDIR=~/vendor/ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/ \
 ANDROID_GST_LIBDIR=~/vendor/gst-android/arm64/lib/ \
    cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  clippy
```

```
(cd android-openxr-exp/example1; \
GST_LIBDIR="$HOME/avrr_tree/gst-android/arm64/lib" \
PKG_CONFIG_ALLOW_CROSS=1 \
PKG_CONFIG_PATH=~/avrr_tree/gst-android/arm64/lib/pkgconfig \
ANDROID_HOME=/opt/android-sdk-linux \
ANDROID_NDK_ROOT=/opt/android-sdk-linux/ndk/21.4.7075529 \
OPENXR_LIBDIR=~/avrr_tree/ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/     \
cargo apk build)
```
```
(adb uninstall rust.glutin_openxr1
adb install -r ./example1/target/debug/apk/glutin-openxr1.apk ) &&
adb shell am start -n rust.glutin_openxr1/android.app.NativeActivity     \
-a android.intent.action.MAIN -c android.intent.category.LAUNCHER
```
