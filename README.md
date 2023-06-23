```
ANDROID_HOME=~/Android/Sdk/ ANDROID_NDK_ROOT=~/Android/Sdk/ndk/25.2.9519653/ OPENXR_LIBDIR=~/vendor/ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/ \
    cargo apk run

ANDROID_HOME=~/Android/Sdk/ ANDROID_NDK_ROOT=~/Android/Sdk/ndk/25.2.9519653/ OPENXR_LIBDIR=~/vendor/ovr_openxr_mobile_sdk/OpenXR/Libs/Android/arm64-v8a/Debug/ \
    cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  clippy
```
