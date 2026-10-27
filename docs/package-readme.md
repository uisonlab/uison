# uison — package for C and C++

**English** · [Русский](https://github.com/MarketKernel/uison/blob/main/docs/ru/package-readme.md)

- `include/uison.h` — the C API (C99); `include/uison_kit.h` — helpers for C (screen JSON in code without escaping, data with substitution, structs ↔ data); `include/uison.hpp` — C++ on top of the C API (C++17, with C++20 conveniences); `include/uison_widgets.hpp` — Flutter-style widgets (C++20).
- `bin/uison-gen` — C++ widgets for your own components (`uison_generate_widgets` in CMake).
- `lib/` — the static (`libuison.a`, `uison.lib`) and dynamic (`libuison.dylib`, `libuison.so`, `uison.dll` + `uison.dll.lib`) library;
  iOS — `uison.xcframework` (`ios-arm64[-simulator]/libuison.a`), Android — `lib/<ABI>/` (building and APK — `docs/mobile.md`); the browser — `lib/libuison.a` for Emscripten and `web/shell.html` (`docs/wasm.md`).
- `lib/cmake/uison` — for CMake: `find_package(uison)` → `uison::uison` (static) and `uison::shared` (dynamic).
- `lib/pkgconfig` — `pkg-config --cflags --libs uison-static` (static) or `uison` (dynamic).

```cmake
list(APPEND CMAKE_PREFIX_PATH /path/to/this/package)
find_package(uison REQUIRED)
target_link_libraries(app PRIVATE uison::uison)
```

Quick start and lessons: https://github.com/MarketKernel/uison/tree/main/docs — the [C](https://github.com/MarketKernel/uison/tree/main/docs/c) and [C++](https://github.com/MarketKernel/uison/tree/main/docs/cpp) series.
