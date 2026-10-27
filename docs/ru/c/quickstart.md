# Быстрый старт: uison в программе на C

[English](../../c/quickstart.md) · **Русский**

Нужны компилятор C, CMake и Rust (он собирает библиотеку) — или готовый пакет вместо Rust ([урок 0](lessons/00-setup.md)).

## 1. Программа

`main.c` — окно с текстом и кнопкой; нажатие увеличивает счётчик:

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

Что здесь происходит — [урок 1](lessons/01-hello.md).

## 2. Сборка

### Способ А — CMake собирает всё сам (рекомендуется)

Рядом с `main.c` — `CMakeLists.txt` (путь к `uison` — ваш):

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

Первая сборка библиотеки — около минуты (Rust компилирует фреймворк), дальше — секунды. В Visual Studio — «Открыть локальную папку» с этим `CMakeLists.txt` ([урок 0](lessons/00-setup.md#visual-studio)).

### Способ Б — одной командой компилятора

Собрать библиотеку один раз:

```sh
cd uison && cargo build --release -p uison-c   # target/release/libuison.a and libuison.dylib / .so
```

Затем программу (`U` — путь к `uison`):

```sh
# macOS
clang -std=c99 main.c -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello

# Linux
gcc -std=c99 main.c -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o hello
```

`-dead_strip` / `--gc-sections` выбрасывают неиспользуемый код библиотеки, `-x,-S` / `-s` — символы: программа ≈ 0,7 МБ на macOS, 0,9 МБ на Linux x86_64 (без них — до 2,8 МБ).

Windows («Командная строка разработчика»; библиотека — `target\release\uison.lib`):

```bat
cl /std:c11 /utf-8 main.c /I %U%\crates\uison-c\include %U%\target\release\uison.lib ^
  dxgi.lib d2d1.lib d3d11.lib dwrite.lib dwmapi.lib user32.lib shell32.lib gdi32.lib winmm.lib ^
  kernel32.lib advapi32.lib ole32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib ^
  /link /OPT:REF /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup
```

`/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup` — окно без консоли при обычном `main()`; без них рядом откроется консоль с выводом `printf` — удобно при отладке. `uison.lib` подходит к любому CRT: `/MT` (так собирает `cl`) — программа запускается на любой Windows 10+; `/MD` (так собирает CMake) — нужен Visual C++ Redistributable. Для раздачи — `/MT` (в CMake: `set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)`).

### Способ В — готовый пакет

Без Rust: пакет `dist/uison-<platform>` из релиза (или `tools/c-dist.sh macos` / `linux` / `ios` / `android` / `wasm`; на Windows — `tools\c-dist.ps1`). В нём заголовки, статическая и динамическая библиотека, файлы для CMake и pkg-config; его можно положить куда угодно (и в свой репозиторий):

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

## 3. Статическая или динамическая

| | статическая (`uison::uison`, `libuison.a`, `uison.lib`) | динамическая (`uison::shared`, `libuison.dylib` / `.so`, `uison.dll`) |
|---|---|---|
| что отдавать пользователю | одну программу | программу и библиотеку рядом |
| размер | программа 0,7–0,9 МБ | программа ≈ 35 КБ + библиотека 0,7–1 МБ |
| когда | почти всегда | несколько программ на одной библиотеке; обновлять библиотеку отдельно |

С динамической программа ищет библиотеку рядом с собой (`@rpath` на macOS, `soname` на Linux): при раздаче положите её в папку программы (на Linux — `-Wl,-rpath,'$ORIGIN'`). На Windows в CMake это делает `uison_copy_dll(hello)`.

## 4. Если что-то не так

- **Ошибка в JSON** — `uison_ui_new` вернёт `NULL`; `uison_kit_fail()` печатает текст ошибки с местом: `near "…{ Text: { txt <here> : 1 }…"`.
- **Кириллица на MSVC** — `/utf-8`.
- **Строки из uison** (`uison_data_str`, `uison_cx_key`) живут до конца обработчика — нужна дольше, скопируйте.
- **Другой поток** не трогает интерфейс напрямую — только `uison_post` ([урок 8](lessons/08-threads.md)).
- **Проверка без окна** — `UISON_SCRIPT=scenario.json ./hello` выполняет нажатия и ввод сам (так проверяются уроки), `UISON_CAPTURE=folder` — снимки окна.

Дальше — [уроки](README.md#уроки).
