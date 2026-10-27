# The browser (WebAssembly)

**English** · [Русский](ru/wasm.md)

The same program runs in the browser: the screen, data and handlers are the same as on the desktop, and the page's canvas does the drawing. The page runs test scenarios (`?script=…`) itself; `tools/web-test.sh` runs them in headless Chrome.

## Rust

A Rust program is built for `wasm32-unknown-unknown` — no wasm-bindgen and no glue code: drawing and input live in `web/uison.js` (about 350 lines), the program is `app.wasm`.

```sh
rustup target add wasm32-unknown-unknown
tools/web.sh studio                            # repository package → target/web/studio/
tools/web.sh path/to/hello.wasm                # your own program: cargo build --release --target wasm32-unknown-unknown
python3 tools/serve.py target/web/studio 8000  # http://localhost:8000/ (?stats=1, ?flash=1, ?script=…)
```

`main` is called from `uison.js`; `uison::run` only stores the app. There are no threads: `Handle::post` works from the same thread. For your own program, see [lesson 11 of the Rust series](rust/lessons/11-everywhere.md).

## C and C++

A C or C++ program runs in the browser too. The screen, data and handlers are the same as on the desktop, and it draws on the page's canvas. You build it the usual C/C++ way, with [Emscripten](https://emscripten.org): `emcc`/`em++` or `emcmake cmake`. The C and C++ standard libraries are all there: `printf`, `malloc`, `std::string`, `std::vector`, exceptions.

```sh
tools/c-dist.sh wasm      # dist/uison-wasm: include/, lib/libuison.a, lib/cmake/uison, web/shell.html
```

You need Rust with the `wasm32-unknown-emscripten` target (`rustup target add wasm32-unknown-emscripten`) — only to build the package itself — and [emsdk](https://emscripten.org/docs/getting_started/downloads.html): `./emsdk install latest && ./emsdk activate latest && source ./emsdk_env.sh`.

On Windows the package is `tools\c-dist.ps1 -Wasm`, and emsdk is `emsdk.ps1 install latest`, `emsdk.ps1 activate latest`, `. .\emsdk_env.ps1` in PowerShell. Its very first run needs Python on `PATH` (later it uses the Python it installs itself); the "python" from the Microsoft Store is only a link to the Store. `emcmake` needs `ninja` (or `mingw32-make`) on `PATH`. Visual Studio has one in `Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja`.

### A single command

```sh
D=dist/uison-wasm
em++ -std=c++20 -Oz -fwasm-exceptions -I$D/include hello.cpp $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
emcc -std=c11 -Oz -I$D/include hello.c $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html   # C
python3 tools/serve.py site 8000         # http://localhost:8000/ (serves .wasm as application/wasm)
```

No other flags are needed. All of uison's JS (drawing, text, mouse, keyboard, clipboard, images) lives inside `libuison.a`, and emcc writes it into `index.js` by itself. `site/` ends up with `index.html`, `index.js` and `index.wasm`.

- `--shell-file` is optional: without it the page is Emscripten's default one, and uison replaces its contents with a full-window canvas. With `shell.html` the Emscripten logo does not flash while loading.
- `-o app.js` instead of `.html` produces just the script: your own page includes it with `<script src="app.js">`, and the canvas is `<canvas id="uison">` (if there is none, uison creates its own full-window canvas).
- C++ needs `-fwasm-exceptions`: `uison.hpp` reports errors with exceptions, and without this flag `throw` stops the program. All modern browsers support WebAssembly exceptions (Safari since 15.2). `-fexceptions` (exceptions via JS) also works, but the program is 20 KB larger and slower.
- `-Oz` makes the program 7% smaller than `-O3`.

### CMake

```sh
emcmake cmake -S . -B build-web -DCMAKE_BUILD_TYPE=MinSizeRel -Duison_DIR=$D/lib/cmake/uison
cmake --build build-web
```

```cmake
find_package(uison REQUIRED)
add_executable(app main.cpp)
target_link_libraries(app PRIVATE uison::uison)   # under Emscripten also adds -fwasm-exceptions
uison_web_page(app)                               # app.html + app.js + app.wasm, shell.html template
uison_web_files(app screen.json photo.jpg)        # files next to the program (see below)
```

The Emscripten toolchain looks for packages only in its own sysroot, so the package path goes through `uison_DIR` (or `set(uison_DIR …)` in CMakeLists.txt). A build from source (`add_subdirectory(path/to/uison/crates/uison-c)`) under `emcmake` picks the `wasm32-unknown-emscripten` target by itself. Outside the browser `uison_web_page` does nothing and `uison_web_files` copies the files next to the program — one CMakeLists.txt for all platforms. `UISON_WASM_EXCEPTIONS=OFF` stops it from adding `-fwasm-exceptions` (for a program without exceptions or with `-fexceptions`).

### Files next to the program

`uison_ui_load("screen.json")`, `image_file("photo.jpg")` and `fopen` read files from the Emscripten file system. `--preload-file` fills it: the files are packed into `app.data` and loaded before `main`.

```sh
em++ … --preload-file screen.json@/screen.json --preload-file photo.jpg@/photo.jpg -o site/index.html
```

The browser loads `Image { src: "photo.jpg" }` by itself, from a URL next to the page, so such a file also goes next to `index.html`. `uison_web_files` in CMake does both.

### How it works

- `libuison.a` for Emscripten is the same code as on the other platforms, plus the `uison-web` backend built for `wasm32-unknown-emscripten`. There is a single allocator (Emscripten's malloc), so Rust and C/C++ share memory.
- The JS lives in the library as `EM_JS` strings — the same way the `EM_JS` macro from `<emscripten/em_js.h>` works. The JS finds the `uison_*` functions it calls (mouse, keyboard and frame events) in the WebAssembly function table, not through exports. So you need neither `-sEXPORTED_FUNCTIONS`, nor `--js-library`, nor `--pre-js`.
- In the browser `uison_run` does not return from `main`: events come from JS, and the `main` stack stays in place. Lambdas that capture `main`'s locals (`[&]`) work just as on the desktop — with any exception mode. The usual Emscripten exit (`emscripten_exit_with_live_runtime`, `emscripten_set_main_loop`) throws a JS exception, and with `-fwasm-exceptions` it runs `main`'s destructors. uison exits with a wasm trap, which C++ cleanup does not catch.
- `uison_post` works in the browser too: the deferred function runs in the browser's next task (there is one thread).
- Emscripten's default settings are fine: the 64 KB stack is enough even for Studio (checked with `-sSTACK_OVERFLOW_CHECK=2`), and so is the 16 MB of memory. If your program needs more data, add `-sALLOW_MEMORY_GROWTH`.

### Limitations

- **No threads**: `std::thread` and `uison_post` from another thread do not work in the browser. Emscripten threads (`-pthread`) need a Rust standard library built with `atomics`, and stable Rust does not have one. `uison_post` from the main thread works.
- **Static library only**: this build has no dynamic linking.
- **`UISON_RELOAD`** (live JSON reload) does not work in the browser.

## Testing

`tools/lessons.sh wasm` builds the C and C++ lessons with `emcmake` and runs their scenarios in headless Chrome. `tools/lessons-compare.sh web` compares screenshots of `form2-c`, `todo-cpp`, `list10k-cpp` with the Rust versions in the browser (they match pixel-for-pixel). `tools/web.sh out/wasm/cpp/todo-cpp.html` puts the finished page into `target/web/todo-cpp/` for `tools/web-test.sh`. Headless tests (`abi-test`, `cpp-test`) run in node: `node abi-test.js`.
