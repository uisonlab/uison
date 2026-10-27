# uison — пакет для C и C++

[English](../package-readme.md) · **Русский**

- `include/uison.h` — C API (C99); `include/uison_kit.h` — помощники для C (JSON экрана в коде без экранирования, данные с подстановкой, структуры ↔ данные); `include/uison.hpp` — C++ поверх C API (C++17, удобства C++20); `include/uison_widgets.hpp` — виджеты как во Flutter (C++20).
- `bin/uison-gen` — виджеты C++ своих компонентов (`uison_generate_widgets` в CMake).
- `lib/` — статическая (`libuison.a`, `uison.lib`) и динамическая (`libuison.dylib`, `libuison.so`, `uison.dll` + `uison.dll.lib`) библиотека;
  iOS — `uison.xcframework` (`ios-arm64[-simulator]/libuison.a`), Android — `lib/<ABI>/` (сборка и APK — `docs/mobile.md`); браузер — `lib/libuison.a` под Emscripten и `web/shell.html` (`docs/wasm.md`).
- `lib/cmake/uison` — для CMake: `find_package(uison)` → `uison::uison` (статическая) и `uison::shared` (динамическая).
- `lib/pkgconfig` — `pkg-config --cflags --libs uison-static` (статическая) или `uison` (динамическая).

```cmake
list(APPEND CMAKE_PREFIX_PATH /path/to/this/package)
find_package(uison REQUIRED)
target_link_libraries(app PRIVATE uison::uison)
```

Быстрый старт и уроки: https://github.com/MarketKernel/uison/tree/main/docs — серии [C](https://github.com/MarketKernel/uison/tree/main/docs/c) и [C++](https://github.com/MarketKernel/uison/tree/main/docs/cpp).
