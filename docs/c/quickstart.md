# Quick start: uison in a C program

**English** · [Русский](../ru/c/quickstart.md)

You need a C compiler, CMake and Rust (it builds the library) — or a prebuilt package instead of Rust ([lesson 0](lessons/00-setup.md)).

## 1. The program

`main.c` is a window with a text and a button; pressing the button increments a counter:

<!-- code: examples-c/lessons/01-hello/main.c -->
```c
/* Lesson 1: a window with a counter. The screen is JSON right in the code (UISON_JSON), the logic is a handler function. */
#include <uison_kit.h>

/* Handler of the "inc" action: read n, write n + 1 — the screen updates itself. */
static void inc(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
}

int main(void) {
    /* data is the data (one number n), root is the element tree; {@n} substitutes n, on.click names the action. */
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [320, 200],
        data: { n: 0 },
        root: { Center: { child: { Column: { gap: 12, cross: "center", children: [
            { Text: { text: "Pressed {@n} times", size: 20 } },
            { Button: { key: "inc", text: "+1", on: { click: "inc" } } },
        ] } } } }
    }));
    if (!ui) return uison_kit_fail(); /* error in the JSON: prints the message and the location */
    UISON_ON(ui, inc);                /* action "inc" → function inc */
    return uison_run(ui, "Lesson 1");
}
```

What happens here is explained in [lesson 1](lessons/01-hello.md).

## 2. Building

### Option A: CMake builds everything (recommended)

Put a `CMakeLists.txt` next to `main.c` (the path to `uison` is yours):

```cmake
cmake_minimum_required(VERSION 3.19)
project(hello C)

add_subdirectory(../uison/crates/uison-c uison)   # CMake runs cargo and builds libuison
add_executable(hello main.c)
target_link_libraries(hello PRIVATE uison::uison) # static; uison::shared — dynamic
if(MSVC)
  target_compile_options(hello PRIVATE /utf-8)   # UTF-8 source (non-ASCII text in strings)
endif()
```

```sh
cmake -S . -B build && cmake --build build
./build/hello            # Windows: build\Debug\hello.exe
```

The first build of the library takes about a minute (Rust compiles the framework), after that — seconds. In Visual Studio, use "Open a local folder" with this `CMakeLists.txt` ([lesson 0](lessons/00-setup.md#visual-studio)).

### Option B: a single compiler command

Build the library once:

```sh
cd uison && cargo build --release -p uison-c   # target/release/libuison.a and libuison.dylib / .so
```

Then the program (`U` is the path to `uison`):

```sh
# macOS
clang -std=c99 main.c -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello

# Linux
gcc -std=c99 main.c -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o hello
```

`-dead_strip` / `--gc-sections` drop unused library code, `-x,-S` / `-s` drop symbols: the program is ≈ 0.7 MB on macOS, 0.9 MB on Linux x86_64 (without them — up to 2.8 MB).

Windows ("Developer Command Prompt"; the library is `target\release\uison.lib`):

```bat
cl /std:c11 /utf-8 main.c /I %U%\crates\uison-c\include %U%\target\release\uison.lib ^
  dxgi.lib d2d1.lib d3d11.lib dwrite.lib dwmapi.lib user32.lib shell32.lib gdi32.lib winmm.lib ^
  kernel32.lib advapi32.lib ole32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib ^
  /link /OPT:REF /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup
```

`/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup` gives you a window without a console with a plain `main()`; without them a console with `printf` output opens next to the window — handy for debugging. `uison.lib` works with either CRT: `/MT` (what `cl` uses) — the program runs on any Windows 10+; `/MD` (what CMake uses) — needs the Visual C++ Redistributable. For distribution use `/MT` (in CMake: `set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)`).

### Option C: a prebuilt package

No Rust: the `dist/uison-<platform>` package from a release (or `tools/c-dist.sh macos` / `linux` / `ios` / `android` / `wasm`; on Windows — `tools\c-dist.ps1`). It contains the headers, the static and dynamic library, and files for CMake and pkg-config; you can put it anywhere (including your own repository):

```cmake
set(uison_DIR /path/to/dist/uison-macos/lib/cmake/uison)
find_package(uison REQUIRED)
target_link_libraries(hello PRIVATE uison::uison)
```

```sh
export PKG_CONFIG_PATH=/path/to/dist/uison-linux-x86_64/lib/pkgconfig
cc main.c $(pkg-config --cflags --libs uison-static) -o hello   # static
cc main.c $(pkg-config --cflags --libs uison) -o hello          # dynamic
```

## 3. Static or dynamic

| | static (`uison::uison`, `libuison.a`, `uison.lib`) | dynamic (`uison::shared`, `libuison.dylib` / `.so`, `uison.dll`) |
|---|---|---|
| what you ship | a single program | the program with the library next to it |
| size | program 0.7–0.9 MB | program ≈ 35 KB + library 0.7–1 MB |
| when | almost always | several programs sharing one library; updating the library separately |

With the dynamic library, the program looks for it next to itself (`@rpath` on macOS, `soname` on Linux): when you distribute it, put the library in the program's folder (on Linux — `-Wl,-rpath,'$ORIGIN'`). On Windows, `uison_copy_dll(hello)` in CMake does this.

## 4. If something goes wrong

- **An error in JSON** — `uison_ui_new` returns `NULL`; `uison_kit_fail()` prints the error text and where it is: `near "…{ Text: { txt <here> : 1 }…"`.
- **Non-ASCII text with MSVC** — `/utf-8`.
- **Strings from uison** (`uison_data_str`, `uison_cx_key`) live until the end of the handler — if you need one longer, copy it.
- **Another thread** never touches the UI directly — only through `uison_post` ([lesson 8](lessons/08-threads.md)).
- **Checking without a window** — `UISON_SCRIPT=scenario.json ./hello` performs the taps and typing by itself (this is how the lessons are checked), `UISON_CAPTURE=folder` takes window screenshots.

Next: [the lessons](README.md#lessons).
