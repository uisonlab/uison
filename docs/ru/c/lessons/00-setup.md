# Урок 0. Подготовка системы

[English](../../../c/lessons/00-setup.md) · **Русский**

**Что будет:** компилятор C, CMake и библиотека uison на вашей машине; в конце — урок 1.

Нужно:
- **компилятор C** — clang (macOS), gcc (Linux), MSVC из Visual Studio (Windows);
- **CMake** 3.19 или новее — собирает уроки и вашу программу;
- **библиотека uison** — её собирает **Rust** из исходников (CMake вызывает его сам), или скачайте **готовый пакет** — тогда Rust не нужен;
- **git** — забрать исходники.

Библиотеки для окон, графики и шрифтов ставить не нужно: uison берёт их у системы.

## macOS

1. Компилятор и git — инструменты командной строки Apple:

   ```sh
   xcode-select --install
   ```

   Если стоит Xcode — они уже есть. Xcode целиком нужен только для iPhone ([урок 10](10-everywhere.md)).
2. CMake: `brew install cmake` ([Homebrew](https://brew.sh)) или `.dmg` с [cmake.org](https://cmake.org/download/) (тогда в терминале — `sudo "/Applications/CMake.app/Contents/bin/cmake-gui" --install`).
3. Rust (собирает библиотеку):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   Enter — установка по умолчанию; потом новое окно терминала.

## Linux

| дистрибутив | команда |
|---|---|
| Ubuntu, Debian, Mint | `sudo apt install build-essential cmake curl git` |
| Fedora | `sudo dnf install gcc cmake curl git` |
| Arch, Manjaro | `sudo pacman -S --needed base-devel cmake curl git` |
| openSUSE | `sudo zypper install gcc cmake curl git` |

Затем Rust — так же, как на macOS. Пакеты `-dev` (Wayland, X11, Cairo, Pango) не нужны: uison загружает эти библиотеки во время работы, программа зависит только от libc. Нужен рабочий стол с Wayland или X11.

## Windows

> Раздел «Visual Studio» проверен вживую с Visual Studio 2026 (открыть папку, цель запуска, отладка — снимки ниже); саму установку на чистой Windows ещё не проходили.

1. **Visual Studio Community** (бесплатно; 2026, подойдёт и 2022): [visualstudio.microsoft.com](https://visualstudio.microsoft.com/ru/downloads/). В установщике на вкладке «Рабочие нагрузки» отметьте **«Разработка классических приложений на C++»**. В неё входят компилятор MSVC, Windows SDK и CMake — отдельно их ставить не нужно. Около 7 ГБ.
2. **Git**: [git-scm.com](https://git-scm.com/download/win) или `winget install Git.Git`.
3. **Rust**: [rustup-init.exe](https://rustup.rs) — Enter (установка по умолчанию). Visual Studio уже стоит, поэтому установщик сразу найдёт MSVC.
4. Откройте новое окно терминала — чтобы подхватились пути. Команды `cl` и `cmake` доступны в **«Командной строке разработчика для VS»** (меню «Пуск»; у VS 2022 в конце названия «2022»), а в Visual Studio — сразу.

### Visual Studio

Visual Studio открывает проект CMake без `.sln`:

1. **Файл → Открыть → Папку…** — папка с `CMakeLists.txt` (для уроков — `examples-c` из репозитория uison). Visual Studio сама запустит CMake; первый раз CMake вызовет cargo и соберёт библиотеку — минута-две, ход виден в окне «Вывод».

   ![Открытая папка](../../../c/img/setup-open-folder.png)

2. На панели инструментов — конфигурация (**x64-Debug** создаётся сама; другие — в `CMakeSettings.json` или `CMakePresets.json`) и **«Выбрать элемент запуска»** — программа, например `c-01-hello.exe` (сначала Visual Studio сама берёт первую программу списка).

   ![Элемент запуска](../../../c/img/setup-startup-item.png)

3. **F5** — собрать и запустить под отладчиком, **Ctrl+F5** — без него. Точка останова в обработчике (`inc` в уроке 1) срабатывает при нажатии кнопки: видно `cx`, данные, стек.

   ![Точка останова](../../../c/img/setup-breakpoint.png)

4. Программа — в `out\build\x64-Debug\` внутри папки проекта.
5. Кириллица в строках: исходники uison — UTF-8, компилятору нужен ключ `/utf-8` (в уроках он уже есть в `CMakeLists.txt`).

Второй путь — без Visual Studio-IDE: «Командная строка разработчика» и одна команда `cl` ([быстрый старт](../quickstart.md#способ-б--одной-командой-компилятора)).

## Готовый пакет вместо Rust

Если Rust ставить не хочется: в релизах репозитория (`nightly` или версия) лежат пакеты `uison-macos`, `uison-linux-x86_64`, `uison-windows-x64`… Распакуйте и укажите CMake путь:

```sh
cmake -S examples-c -B build -DUISON_PACKAGE=/path/to/uison-macos
```

## Редактор

- **Visual Studio** (Windows) — см. выше.
- **VS Code** + расширения **C/C++** и **CMake Tools**: «CMake: Select Kit» (компилятор), «CMake: Build», F5.
- **CLion** — открывает `CMakeLists.txt` как проект.

## Проверка

```sh
cc --version        # Windows: cl (in the "Developer Command Prompt")
cmake --version     # 3.19 or newer
rustc --version     # not needed if you took a prebuilt package
git --version
```

Урок 1 из репозитория uison:

```sh
git clone https://github.com/MarketKernel/uison
cd uison
cmake -S examples-c -B out/c/build -DCMAKE_BUILD_TYPE=Release
cmake --build out/c/build --target c-01-hello --config Release
./out/c/build/c-01-hello          # Windows: out\c\build\Release\c-01-hello.exe
```

Откроется окно с кнопкой «+1».

![Урок 1](../../../c/img/01-hello.png)

Что-то не так:
- `Could not find UISON_CARGO` — Rust не поставлен или окно терминала открыто до установки; либо возьмите готовый пакет (`-DUISON_PACKAGE=…`);
- `No CMAKE_C_COMPILER could be found` — нет компилятора: на macOS — `xcode-select --install`, на Linux — `build-essential` / `gcc`, на Windows — нагрузка «Разработка классических приложений на C++»;
- окно не открылось на Linux, в терминале ошибка `Wayland: …` — программа запущена без рабочего стола (по SSH, в контейнере).

Дальше — [урок 1: окно и кнопка](01-hello.md).
