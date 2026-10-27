# Quick start: uison in a C++ program

**English** · [Русский](../ru/cpp/quickstart.md)

You need a C++20 compiler, CMake and Rust (it builds the library) — or a prebuilt package instead of Rust ([lesson 0](lessons/00-setup.md)).

## 1. The program

`main.cpp` — a window with a text and a button; pressing the button increments a counter:

<!-- code: examples-cpp/lessons/01-hello/main.cpp -->
```cpp
// Lesson 1: a window with a counter. The screen is C++ widgets (like in Flutter), the handler is a lambda right on the button.
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    // Center → Column → Text + Button: the same tree as { Center: { child: { Column: … } } } in JSON.
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        // {@n} substitutes the value of n from data; the screen updates itself when n changes.
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Lesson 1");
}
```

What is going on here — see [lesson 1](lessons/01-hello.md).

## 2. Building

Put a `CMakeLists.txt` next to `main.cpp` (use your own path to `uison`):

```cmake
cmake_minimum_required(VERSION 3.19)
project(hello CXX)
set(CMAKE_CXX_STANDARD 20)

add_subdirectory(../uison/crates/uison-c uison)   # CMake runs cargo and builds libuison
add_executable(hello main.cpp)
target_link_libraries(hello PRIVATE uison::uison) # static; uison::shared is dynamic
if(MSVC)
  target_compile_options(hello PRIVATE /utf-8)   # the source is UTF-8 (non-ASCII text in strings)
endif()
```

```sh
cmake -S . -B build && cmake --build build
./build/hello            # Windows: build\Debug\hello.exe
```

The first build of the library takes about a minute (Rust compiles the framework); after that, seconds. In Visual Studio, use "Open a local folder" with this `CMakeLists.txt` ([lesson 0](lessons/00-setup.md#visual-studio)).

A single compiler command without CMake (the library is built beforehand: `cargo build --release -p uison-c` in `uison`; `U` is the path to it):

```sh
# macOS
clang++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello

# Linux
g++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o hello
```

Windows, a prebuilt package instead of Rust, static or dynamic linking, phones and the browser — see [lesson 12](lessons/12-build.md).

## 3. If something goes wrong

- **An error in the screen description** — `uison::Ui` throws `uison::Error` with the location: `Button at root/0/1: unknown parameter 'txt'`.
- **Widget fields go in declaration order**: `Button("+1", {.icon = …, .key = …, .on_click = …})`; out of order is an error in GCC and MSVC. The order is the same everywhere: the widget's own properties → sizes → common ones (`key`, `visible`…) → events → `child`/`children`; IDE tooltips show the fields.
- **`)"` inside a raw string** ends `R"( … )"` early — write `R"json( … )json"`.
- **Non-ASCII text on MSVC** — `/utf-8`.
- **Another thread** must not touch the UI directly — only through `handle.post(…)` ([lesson 9](lessons/09-threads.md)).
- **Checking without a window** — `UISON_SCRIPT=scenario.json ./hello` performs the presses and typing by itself, `UISON_CAPTURE=folder` saves screenshots of the window.

Next: [the lessons](README.md#lessons).
