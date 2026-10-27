# iOS and Android

**English** · [Русский](ru/mobile.md)

It is the same program as on the desktop: `main()` creates the screen and calls `run`. Touch, the on-screen keyboard, the safe area and dark theme work out of the box. Only the packaging differs: on iOS the program lives in an `.app`, on Android it is `libmain.so` inside an APK. No Xcode project and no Gradle: the `tools/ios.sh` and `tools/android.sh` scripts build the `.app` and the APK.

The app runs test scenarios (`UISON_SCRIPT`) itself. The `tools/ios-test.sh` and `tools/android-test.sh` scripts take the screenshots.

## Rust

Repository examples are referenced by package name, your own program by its path ([lesson 11 of the Rust series](rust/lessons/11-everywhere.md)):

```sh
tools/ios-run.sh studio sim                       # build for the simulator, make the .app, install, launch
tools/ios-test.sh studio scenarios/touch.json sim # the same with a scenario and screenshots
tools/android-run.sh studio                       # APK (arm64-v8a and armeabi-v7a), install, launch
tools/android-test.sh studio scenarios/studio-phone.json
```

You need: the Rust targets (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi`), Xcode for iOS, the Android SDK and NDK for Android (see below). On Android a Rust program is built as `libmain.so` (`-C link-arg=-shared`); the script does this for you.

## C and C++: packages

```sh
tools/c-dist.sh ios       # dist/uison-ios: uison.xcframework (inside: ios-arm64[-simulator]/libuison.a)
tools/c-dist.sh android   # dist/uison-android: lib/<ABI>/libuison.a and libuison.so (arm64-v8a, armeabi-v7a)
```

You need: Rust with the targets (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi`). For iOS, Xcode. For Android, the SDK and NDK (Android Studio → SDK Manager); the scripts look for them in `$ANDROID_HOME`, by default `~/Library/Android/sdk`.

## iOS

The program is a single static build. There is no dynamic library for iOS.

```sh
D=dist/uison-ios
clang -target arm64-apple-ios17.0-simulator -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) \
  hello.c -I$D/include $D/uison.xcframework/ios-arm64-simulator/libuison.a \
  -framework UIKit -framework QuartzCore -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello
```

For a phone, use `-target arm64-apple-ios17.0`, `--sdk iphoneos` and `uison.xcframework/ios-arm64`.

Package it into an `.app`, install and launch; the program's files (JSON, images) go next to it in the `.app`:

```sh
UISON_IOS_FILES="screen.json components.json" tools/ios-run.sh ./hello sim      # simulator
UISON_IOS_FILES="screen.json components.json" tools/ios-run.sh ./hello device   # phone (needs signing)
tools/ios-test.sh ./hello scenario.json sim                                   # scenario and screenshots
```

**Xcode.** Drag `uison.xcframework` into the project (Frameworks, Libraries…), add `include/` to Header Search Paths, and `-lobjc` to Other Linker Flags. Info.plist needs a scene with the `UisonSceneDelegate` delegate (see `tools/ios.sh` for an example). This path has not been tested yet; clang, CMake and `tools/ios.sh` have.

**CMake** (from source or from the package, `-Duison_DIR=dist/uison-ios/lib/cmake/uison`):

```sh
cmake -S . -B build-ios -DCMAKE_SYSTEM_NAME=iOS -DCMAKE_OSX_SYSROOT=iphonesimulator \
  -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_OSX_DEPLOYMENT_TARGET=17.0
cmake --build build-ios     # build-ios/hello.app/hello → tools/ios-run.sh
```

## Android

The program is built as a shared library, `libmain.so`. The uison Java layer (window, input, keyboard) loads it, and the `JNI_OnLoad` entry point from `libuison.a` calls your `main()`.

```sh
NDK=~/Library/Android/sdk/ndk/<version>; CC=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin   # Linux: linux-x86_64
D=dist/uison-android
mkdir -p app/arm64-v8a app/armeabi-v7a
$CC/aarch64-linux-android29-clang -shared -fPIC hello.c -I$D/include $D/lib/arm64-v8a/libuison.a \
  -llog -landroid -ldl -lm -Wl,--gc-sections,--version-script=$D/lib/cmake/uison/uison-android.map -s \
  -o app/arm64-v8a/libmain.so
$CC/armv7a-linux-androideabi29-clang -shared -fPIC hello.c -I$D/include $D/lib/armeabi-v7a/libuison.a \
  -llog -landroid -ldl -lm -Wl,--gc-sections,--version-script=$D/lib/cmake/uison/uison-android.map -s \
  -o app/armeabi-v7a/libmain.so
```

For C++, use `clang++` from the same folder. `--version-script` exports only `JNI_OnLoad` and `Java_*`: unused `uison_*` functions are dropped, which saves 26 KB.

An APK without Gradle: the Java layer, the manifest, signing with the debug key. The app name comes from the folder (`app`):

```sh
UISON_ANDROID_ASSETS="screen.json components.json" tools/android-run.sh app   # install and launch
tools/android-test.sh app scenario.json                                      # scenario and screenshots
```

On first launch the app copies the files from assets into its own folder and makes it the current directory. So `uison_ui_load("screen.json")` and `image_file("photo.jpg")` work just as on the desktop.

**CMake** with the NDK toolchain. The program is `add_library(main SHARED …)`; the `uison::uison` target adds the flags for `libmain.so` itself:

```cmake
find_package(uison REQUIRED)               # -Duison_DIR=dist/uison-android/lib/cmake/uison
add_library(main SHARED main.cpp)
target_link_libraries(main PRIVATE uison::uison)
```

```sh
cmake -S . -B build-android -DCMAKE_TOOLCHAIN_FILE=$NDK/build/cmake/android.toolchain.cmake \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=29 -DCMAKE_BUILD_TYPE=Release -Duison_DIR=$PWD/dist/uison-android/lib/cmake/uison
cmake --build build-android   # build-android/libmain.so → app/arm64-v8a/ (llvm-strip) → tools/android.sh
```

The iOS and NDK toolchains look for packages only inside their own sysroot, so the package path is set with `uison_DIR`, not `CMAKE_PREFIX_PATH`. When building from source (`add_subdirectory`), the Rust target is picked automatically: from `CMAKE_OSX_SYSROOT` on iOS and from the ABI on Android.

## Sizes

| | program | result |
|---|---|---|
| iOS, lesson "A window and a button" (C) | 670 KB | `.app` |
| Android, lesson "Screens from files" (C) | 716 KB arm64-v8a, 544 KB armeabi-v7a | APK 748 KB |
| Android, lesson "Threads" (C++, `libc++` statically) | 872 KB arm64-v8a | APK 448 KB |
