# iOS и Android

[English](../mobile.md) · **Русский**

Та же программа, что и на компьютере: `main()` создаёт экран и вызывает `run`. Касания, экранная клавиатура, безопасная зона и тёмная тема работают сами. Отличается только упаковка: на iOS программа лежит в `.app`, на Android — это `libmain.so` внутри APK. Без Xcode-проекта и без Gradle: `.app` и APK собирают скрипты `tools/ios.sh` и `tools/android.sh`.

Сценарии проверки (`UISON_SCRIPT`) выполняет само приложение. Скрипты `tools/ios-test.sh` и `tools/android-test.sh` снимают экран.

## Rust

Примеры репозитория — по имени пакета, своя программа — путём к ней ([урок 11 серии Rust](rust/lessons/11-everywhere.md)):

```sh
tools/ios-run.sh studio sim                       # build for the simulator, make the .app, install, launch
tools/ios-test.sh studio scenarios/touch.json sim # the same with a scenario and screenshots
tools/android-run.sh studio                       # APK (arm64-v8a and armeabi-v7a), install, launch
tools/android-test.sh studio scenarios/studio-phone.json
```

Что нужно: таргеты Rust (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi`), Xcode для iOS, Android SDK и NDK для Android (ниже). На Android программа Rust собирается как `libmain.so` (`-C link-arg=-shared`), это скрипт делает сам.

## C и C++: пакеты

```sh
tools/c-dist.sh ios       # dist/uison-ios: uison.xcframework (inside: ios-arm64[-simulator]/libuison.a)
tools/c-dist.sh android   # dist/uison-android: lib/<ABI>/libuison.a and libuison.so (arm64-v8a, armeabi-v7a)
```

Что нужно: Rust с таргетами (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi`). Для iOS — Xcode. Для Android — SDK и NDK (Android Studio → SDK Manager); скрипты ищут их в `$ANDROID_HOME`, по умолчанию `~/Library/Android/sdk`.

## iOS

Программа — одна статическая сборка. Динамической библиотеки для iOS нет.

```sh
D=dist/uison-ios
clang -target arm64-apple-ios17.0-simulator -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) \
  hello.c -I$D/include $D/uison.xcframework/ios-arm64-simulator/libuison.a \
  -framework UIKit -framework QuartzCore -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello
```

Для телефона — `-target arm64-apple-ios17.0`, `--sdk iphoneos` и `uison.xcframework/ios-arm64`.

Упаковать в `.app`, установить и запустить; файлы программы (JSON, картинки) кладутся рядом с ней в `.app`:

```sh
UISON_IOS_FILES="screen.json components.json" tools/ios-run.sh ./hello sim      # simulator
UISON_IOS_FILES="screen.json components.json" tools/ios-run.sh ./hello device   # phone (needs signing)
tools/ios-test.sh ./hello scenario.json sim                                   # scenario and screenshots
```

**Xcode.** Перетащите `uison.xcframework` в проект (Frameworks, Libraries…), добавьте `include/` в Header Search Paths, а в Other Linker Flags — `-lobjc`. В Info.plist нужна сцена с делегатом `UisonSceneDelegate` (пример — в `tools/ios.sh`). Этот путь пока не проверялся; проверены clang, CMake и `tools/ios.sh`.

**CMake** (из исходников или из пакета, `-Duison_DIR=dist/uison-ios/lib/cmake/uison`):

```sh
cmake -S . -B build-ios -DCMAKE_SYSTEM_NAME=iOS -DCMAKE_OSX_SYSROOT=iphonesimulator \
  -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_OSX_DEPLOYMENT_TARGET=17.0
cmake --build build-ios     # build-ios/hello.app/hello → tools/ios-run.sh
```

## Android

Программа собирается как разделяемая библиотека `libmain.so`. Java-прослойка uison (окно, ввод, клавиатура) загружает её, а точка входа `JNI_OnLoad` из `libuison.a` вызывает ваш `main()`.

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

На C++ — `clang++` из той же папки. `--version-script` оставляет снаружи только `JNI_OnLoad` и `Java_*`: неиспользуемые функции `uison_*` выбрасываются, это −26 КБ.

APK без Gradle: Java-прослойка, манифест, подпись отладочным ключом. Имя приложения берётся по папке (`app`):

```sh
UISON_ANDROID_ASSETS="screen.json components.json" tools/android-run.sh app   # install and launch
tools/android-test.sh app scenario.json                                      # scenario and screenshots
```

Файлы из assets приложение при первом запуске копирует в свою папку и делает её текущей. Поэтому `uison_ui_load("screen.json")` и `image_file("photo.jpg")` работают как на компьютере.

**CMake** с тулчейном NDK. Программа — `add_library(main SHARED …)`, флаги для `libmain.so` цель `uison::uison` добавляет сама:

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

Тулчейны iOS и NDK ищут пакеты только внутри своего sysroot, поэтому путь к пакету задаётся через `uison_DIR`, а не `CMAKE_PREFIX_PATH`. При сборке из исходников (`add_subdirectory`) таргет Rust выбирается сам: по `CMAKE_OSX_SYSROOT` на iOS и по ABI на Android.

## Размеры

| | программа | итог |
|---|---|---|
| iOS, урок «Окно и кнопка» (C) | 670 КБ | `.app` |
| Android, урок «Экран из файлов» (C) | 716 КБ arm64-v8a, 544 КБ armeabi-v7a | APK 748 КБ |
| Android, урок «Потоки» (C++, `libc++` статически) | 872 КБ arm64-v8a | APK 448 КБ |
