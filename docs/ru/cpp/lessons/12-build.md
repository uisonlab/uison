# Урок 12. Сборка, телефоны, браузер

[English](../../../cpp/lessons/12-build.md) · **Русский**

**Что будет:** как собрать программу всеми способами — CMake из исходников и из готового пакета, одной командой компилятора, статически и динамически — и та же программа на iPhone, Android и в браузере.

## CMake из исходников

```cmake
add_subdirectory(../uison/crates/uison-c uison)   # CMake calls cargo and builds libuison
add_executable(app main.cpp)
target_link_libraries(app PRIVATE uison::uison)   # static; uison::shared — dynamic
```

Нужен Rust. Цель `uison::uison` добавляет всё нужное сама: системные библиотеки, `-dead_strip` / `--gc-sections`, на Android — точку входа, в браузере — исключения WebAssembly. Для своих компонентов — `uison_generate_widgets` ([урок 6](06-components.md)).

## CMake из пакета

Без Rust: пакет `uison-<platform>` из релиза (или `tools/c-dist.sh macos` / `linux` / `ios` / `android` / `wasm`, на Windows — `tools\c-dist.ps1`). В нём заголовки, статическая и динамическая библиотека, `bin/uison-gen`, файлы для CMake и pkg-config.

```cmake
set(uison_DIR /path/to/uison-macos/lib/cmake/uison)   # the iOS, NDK and Emscripten toolchains find packages only this way
find_package(uison REQUIRED)
target_link_libraries(app PRIVATE uison::uison)
```

## Одной командой

Библиотека собрана заранее (`cargo build --release -p uison-c` в репозитории uison, `U` — путь к нему):

```sh
# macOS
clang++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o app

# Linux
g++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o app
```

`-dead_strip` / `--gc-sections` выбрасывают неиспользуемый код библиотеки, `-x,-S` / `-s` — символы: программа ≈ 0,75 МБ.

### Windows одной командой

«Командная строка разработчика для VS»; библиотека — `target\release\uison.lib`:

```bat
cl /std:c++20 /utf-8 /EHsc main.cpp /I %U%\crates\uison-c\include %U%\target\release\uison.lib ^
  dxgi.lib d2d1.lib d3d11.lib dwrite.lib dwmapi.lib user32.lib shell32.lib gdi32.lib winmm.lib ^
  kernel32.lib advapi32.lib ole32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib ^
  /link /OPT:REF /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup
```

- `/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup` — окно без консоли при обычном `main()`; без них рядом откроется консоль с выводом — удобно при отладке. В CMake — `target_link_options(app PRIVATE /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup)`.
- `uison.lib` подходит к любому CRT: `/MT` (так собирает `cl`) — программа запускается на любой Windows 10+; `/MD` (так собирает CMake) — нужен Visual C++ Redistributable. Для раздачи — `/MT` (`set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)`).

## Статическая или динамическая

| | статическая (`uison::uison`) | динамическая (`uison::shared`) |
|---|---|---|
| что отдавать | одну программу | программу и `libuison.dylib` / `.so` / `uison.dll` рядом |
| размер | ≈ 0,75 МБ | программа ≈ 35 КБ + библиотека 0,7–1 МБ |
| когда | почти всегда | несколько программ на одной библиотеке |

С динамической программа ищет библиотеку рядом с собой (`@rpath` на macOS, `-Wl,-rpath,'$ORIGIN'` на Linux). На Windows `uison_copy_dll(app)` кладёт `uison.dll` рядом с `app.exe` после сборки.

## iPhone и iPad

Только статическая библиотека; пакет `uison-ios` — `uison.xcframework`.

```sh
cmake -S . -B build-ios -DCMAKE_SYSTEM_NAME=iOS -DCMAKE_OSX_SYSROOT=iphonesimulator \
  -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_OSX_DEPLOYMENT_TARGET=17.0
cmake --build build-ios
tools/ios-run.sh build-ios/app.app/app sim       # .app → simulator; output goes here
```

Одна команда `clang++`, телефон и подпись, Xcode — [телефоны](../../mobile.md#ios).

## Android

Программа — `libmain.so`: `add_library(main SHARED main.cpp)` вместо `add_executable`, флаги точки входа `uison::uison` добавляет сам.

```sh
cmake -S . -B build-android -DCMAKE_TOOLCHAIN_FILE=$NDK/build/cmake/android.toolchain.cmake \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=29 -DCMAKE_BUILD_TYPE=Release -Duison_DIR=$PWD/dist/uison-android/lib/cmake/uison
cmake --build build-android
mkdir -p app/arm64-v8a && cp build-android/libmain.so app/arm64-v8a/
tools/android-run.sh app                         # APK → phone, launch
```

libc++ NDK компонуется статически (`ANDROID_STL=c++_static` по умолчанию) — программа не тянет `libc++_shared.so`. Подробно — [телефоны](../../mobile.md#android).

## Браузер

[Emscripten](https://emscripten.org): `em++` одной командой или `emcmake`.

```sh
D=dist/uison-wasm                                # tools/c-dist.sh wasm
em++ -std=c++20 -Oz -fwasm-exceptions -I$D/include main.cpp $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
```

```cmake
uison_web_page(app)                  # app.html + app.js + app.wasm; outside the browser — nothing
uison_web_files(app screen.json)     # files next to the program go into the page's file system
```

`-fwasm-exceptions` нужен: `uison.hpp` сообщает об ошибках исключениями. Потоков в браузере нет. Подробно — [браузер](../../wasm.md).

## Иконка программы

Иконка работающего окна (Dock на macOS, заголовок и панель задач на Windows, панель задач на X11) — PNG, который задаётся до `run`:

```cpp
#include "icon.h" // xxd -i icon.png > icon.h: unsigned char icon_png[], unsigned int icon_png_len
uison::set_icon(icon_png, icon_png_len);
return ui.run("Hello");
```

Иконка самого файла программы — дело упаковки. На Windows в исходники программы добавляется сценарий ресурсов — `add_executable(app main.cpp app.rc)`, где `app.rc` — одна строка `1 ICON "icon.ico"` (номер 1 берёт окно); на macOS — пакет `.app` с `.icns`; на Linux — файл `.desktop` и иконка в теме `hicolor` (на Wayland окно находит её по своему `app_id`, `dev.uison.<программа>`). Готовый пример для macOS и Linux — [`bundle.sh`](../../../../examples/disk-usage/bundle.sh) из Disk Usage.

Конец серии. Справочник — [язык экранов](../../screens/README.md), [элементы](../../screens/elements.md), API — [`uison.hpp`](../../../../crates/uison-c/include/uison.hpp).
