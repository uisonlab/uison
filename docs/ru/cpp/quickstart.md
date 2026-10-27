# Быстрый старт: uison в программе на C++

[English](../../cpp/quickstart.md) · **Русский**

Нужны компилятор C++20, CMake и Rust (он собирает библиотеку) — или готовый пакет вместо Rust ([урок 0](lessons/00-setup.md)).

## 1. Программа

`main.cpp` — окно с текстом и кнопкой; нажатие увеличивает счётчик:

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

Что здесь происходит — [урок 1](lessons/01-hello.md).

## 2. Сборка

Рядом с `main.cpp` — `CMakeLists.txt` (путь к `uison` — ваш):

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

Первая сборка библиотеки — около минуты (Rust компилирует фреймворк), дальше — секунды. В Visual Studio — «Открыть локальную папку» с этим `CMakeLists.txt` ([урок 0](lessons/00-setup.md#visual-studio)).

Одна команда компилятора без CMake (библиотека собрана заранее: `cargo build --release -p uison-c` в `uison`, `U` — путь к нему):

```sh
# macOS
clang++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello

# Linux
g++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o hello
```

Windows, готовый пакет вместо Rust, статическая или динамическая, телефоны и браузер — [урок 12](lessons/12-build.md).

## 3. Если что-то не так

- **Ошибка в описании экрана** — `uison::Ui` бросает `uison::Error` с местом: `Button at root/0/1: unknown parameter 'txt'`.
- **Поля виджета — по порядку объявления**: `Button("+1", {.icon = …, .key = …, .on_click = …})`; не по порядку — ошибка GCC и MSVC. Порядок у всех одинаковый: свои свойства → размеры → общие (`key`, `visible`…) → события → `child`/`children`; подсказки IDE показывают поля.
- **`)"` внутри сырой строки** обрывает `R"( … )"` — пишите `R"json( … )json"`.
- **Кириллица на MSVC** — `/utf-8`.
- **Другой поток** не трогает интерфейс напрямую — только `handle.post(…)` ([урок 9](lessons/09-threads.md)).
- **Проверка без окна** — `UISON_SCRIPT=scenario.json ./hello` выполняет нажатия и ввод сам, `UISON_CAPTURE=folder` — снимки окна.

Дальше — [уроки](README.md#уроки).
