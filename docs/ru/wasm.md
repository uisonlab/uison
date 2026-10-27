# Браузер (WebAssembly)

[English](../wasm.md) · **Русский**

Та же программа работает в браузере: экран, данные и обработчики — те же, что на компьютере, рисует canvas страницы. Сценарии проверки (`?script=…`) выполняет страница сама, `tools/web-test.sh` гоняет их в headless Chrome.

## Rust

Программа на Rust собирается под `wasm32-unknown-unknown` — без wasm-bindgen и без обвязки: рисование и ввод — `crates/uison-web/uison.js` (около 350 строк), программа — `app.wasm`.

```sh
rustup target add wasm32-unknown-unknown
tools/web.sh studio                            # repository package → target/web/studio/
tools/web.sh path/to/hello.wasm                # your own program: cargo build --release --target wasm32-unknown-unknown
python3 tools/serve.py target/web/studio 8000  # http://localhost:8000/ (?stats=1, ?flash=1, ?script=…)
```

`main` вызывается из `uison.js`, `uison::run` только запоминает приложение. Потоков нет: `Handle::post` работает из того же потока. Своя программа — [урок 11 серии Rust](rust/lessons/11-everywhere.md).

## C и C++

Программа на C или C++ работает и в браузере. Экран, данные и обработчики те же, что на компьютере, а рисует она на canvas страницы. Собирается программа обычным для C/C++ способом — через [Emscripten](https://emscripten.org): `emcc`/`em++` или `emcmake cmake`. Стандартная библиотека C и C++ на месте: `printf`, `malloc`, `std::string`, `std::vector`, исключения.

```sh
tools/c-dist.sh wasm      # dist/uison-wasm: include/, lib/libuison.a, lib/cmake/uison, web/shell.html
```

Нужны Rust с таргетом `wasm32-unknown-emscripten` (`rustup target add wasm32-unknown-emscripten`) — только чтобы собрать сам пакет — и [emsdk](https://emscripten.org/docs/getting_started/downloads.html): `./emsdk install latest && ./emsdk activate latest && source ./emsdk_env.sh`.

На Windows пакет собирает `tools\c-dist.ps1 -Wasm`, а emsdk ставится так: `emsdk.ps1 install latest`, `emsdk.ps1 activate latest`, `. .\emsdk_env.ps1` в PowerShell. Для самого первого запуска нужен Python в `PATH` (дальше emsdk берёт Python, который поставил сам); «python» из Microsoft Store — только ссылка на Магазин. `emcmake` нужен `ninja` (или `mingw32-make`) в `PATH`. В Visual Studio он есть: `Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja`.

### Одной командой

```sh
D=dist/uison-wasm
em++ -std=c++20 -Oz -fwasm-exceptions -I$D/include hello.cpp $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
emcc -std=c11 -Oz -I$D/include hello.c $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html   # C
python3 tools/serve.py site 8000         # http://localhost:8000/ (serves .wasm as application/wasm)
```

Других флагов не нужно. Весь JS uison (рисование, текст, мышь, клавиатура, буфер обмена, картинки) лежит внутри `libuison.a`, и emcc сам вписывает его в `index.js`. В `site/` получаются `index.html`, `index.js` и `index.wasm`.

- `--shell-file` можно не указывать: тогда страница — стандартная от Emscripten, а uison заменит её содержимое холстом на всё окно. С `shell.html` нет мелькания логотипа Emscripten при загрузке.
- `-o app.js` вместо `.html` — только скрипт: своя страница подключает его через `<script src="app.js">`, холст — `<canvas id="uison">` (если его нет, uison создаст свой на всё окно).
- `-fwasm-exceptions` нужен C++: `uison.hpp` сообщает об ошибках исключениями, а без этого флага `throw` останавливает программу. Исключения WebAssembly поддерживают все современные браузеры (Safari — с 15.2). `-fexceptions` (исключения через JS) тоже работает, но программа больше на 20 КБ и медленнее.
- `-Oz` даёт программу на 7 % меньше, чем `-O3`.

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

Тулчейн Emscripten ищет пакеты только в своём sysroot, поэтому путь к пакету — через `uison_DIR` (или `set(uison_DIR …)` в CMakeLists.txt). Сборка из исходников (`add_subdirectory(path/to/uison/crates/uison-c)`) под `emcmake` сама берёт таргет `wasm32-unknown-emscripten`. Вне браузера `uison_web_page` ничего не делает, а `uison_web_files` копирует файлы к программе — один CMakeLists.txt на все платформы. `UISON_WASM_EXCEPTIONS=OFF` — не добавлять `-fwasm-exceptions` (программа без исключений или с `-fexceptions`).

### Файлы рядом с программой

`uison_ui_load("screen.json")`, `image_file("photo.jpg")` и `fopen` читают файлы из файловой системы Emscripten. Её заполняет `--preload-file`: файлы упаковываются в `app.data` и загружаются до `main`.

```sh
em++ … --preload-file screen.json@/screen.json --preload-file photo.jpg@/photo.jpg -o site/index.html
```

`Image { src: "photo.jpg" }` браузер грузит сам, по адресу рядом со страницей, — такой файл кладётся и рядом с `index.html`. `uison_web_files` в CMake делает и то и другое.

### Как это устроено

- `libuison.a` для Emscripten — тот же код, что на других платформах, плюс бэкенд `uison-web`, собранный под `wasm32-unknown-emscripten`. Аллокатор один (malloc из Emscripten), поэтому память Rust и C/C++ общая.
- JS лежит в библиотеке строками `EM_JS` — так же, как макрос `EM_JS` из `<emscripten/em_js.h>`. Функции `uison_*`, которые зовёт JS (события мыши, клавиатуры, кадры), он находит в таблице функций WebAssembly, а не через экспорты. Поэтому не нужен ни `-sEXPORTED_FUNCTIONS`, ни `--js-library`, ни `--pre-js`.
- `uison_run` в браузере не возвращается из `main`: события приходят из JS, а стек `main` остаётся на месте. Лямбды, которые ссылаются на локальные переменные `main` (`[&]`), работают как на компьютере — с любым режимом исключений. Обычный выход Emscripten (`emscripten_exit_with_live_runtime`, `emscripten_set_main_loop`) бросает JS-исключение, и с `-fwasm-exceptions` оно запускает деструкторы `main`. uison выходит ловушкой wasm, которую очистка C++ не ловит.
- `uison_post` работает и в браузере: отложенная функция выполняется в следующей задаче браузера (поток один).
- Настройки Emscripten по умолчанию подходят: стека 64 КБ хватает даже Studio (проверено с `-sSTACK_OVERFLOW_CHECK=2`), памяти 16 МБ — тоже. Если программе нужно больше данных, добавьте `-sALLOW_MEMORY_GROWTH`.

### Ограничения

- **Потоков нет**: `std::thread` и `uison_post` из другого потока в браузере не работают. Потоки Emscripten (`-pthread`) требуют стандартную библиотеку Rust, собранную с `atomics`, а её нет в стабильном Rust. `uison_post` из главного потока работает.
- **Только статическая библиотека**: динамической компоновки у этой сборки нет.
- **`UISON_RELOAD`** (живая перезагрузка JSON) в браузере не работает.

## Проверка

`tools/lessons.sh wasm` собирает уроки C и C++ через `emcmake` и прогоняет их сценарии в headless Chrome. `tools/lessons-compare.sh web` сравнивает снимки `form2-c`, `todo-cpp`, `list10k-cpp` с Rust-версиями в браузере (совпадают до пикселя). `tools/web.sh out/wasm/cpp/todo-cpp.html` кладёт готовую страницу в `target/web/todo-cpp/` для `tools/web-test.sh`. Тесты без окна (`abi-test`, `cpp-test`) запускаются в node: `node abi-test.js`.
