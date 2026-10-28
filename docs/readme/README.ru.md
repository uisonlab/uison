# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
<a href="README.es.md">🇪🇸 Español</a> ·
<a href="README.fr.md">🇫🇷 Français</a> ·
<a href="README.ar.md">🇸🇦 العربية</a> ·
<a href="README.bn.md">🇧🇩 বাংলা</a> ·
<a href="README.pt.md">🇧🇷 Português</a> ·
<b>🇷🇺 Русский</b> ·
<a href="README.ur.md">🇵🇰 اردو</a> ·
<a href="README.id.md">🇮🇩 Bahasa Indonesia</a> ·
<a href="README.de.md">🇩🇪 Deutsch</a> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) — маленький быстрый UI-фреймворк на Rust вообще без зависимостей:
только стандартная библиотека, вплоть до собственного парсера JSON. Экран описывается
деревом элементов: в JSON-файле, на Rust DSL, виджетами C++ в духе Flutter или JSON прямо
в коде на C. Логика — обычный код на Rust, C или C++: обработчики, данные, потоки. Окно,
раскладку, отрисовку и ввод берёт на себя uison, и одна и та же программа работает на macOS,
Windows, Linux (Wayland и X11), iOS, Android и в браузере.

Каждый бэкенд обращается к системе напрямую, через FFI, написанный вручную: AppKit и CoreText,
Win32 с Direct2D и DirectWrite (COM вручную), протокол Wayland, реализованный с нуля, и X11,
UIKit, Canvas Android через JNI и canvas 2D из WebAssembly. Release-программа — один файл
примерно 0,7–0,9 МБ, которому нужны только библиотеки самой системы, а событие доходит до
экрана на компьютере примерно за миллисекунду.

![Программы на uison: Studio (почта, настройки, графики), галерея элементов, Todo, Disk Usage и маленький браузер](../img/showcase.jpg)

## Как это устроено

```text
экран (JSON, Rust DSL, виджеты C++, UISON_JSON в C)  +  данные  +  обработчики на вашем языке
        │  привязки, шаблоны, темы, раскладка (uison-ui)
        ▼
кадр примитивов: rect, line, circle, tri, poly, text, image, clip, hit
        │  сравнение с прошлым кадром → перерисовать только изменившееся
        ▼
бэкенд системы: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Экран компилируется в *кадр* — плоский список примитивов нескольких видов: прямоугольники,
линии, круги, многоугольники, однострочный текст, картинки, области обрезки и невидимые
области для мыши и касаний. Каждый новый кадр сравнивается с прошлым, и бэкенд перерисовывает
только изменившиеся области. Бэкенд получается маленьким: ему нужно лишь рисовать эти
примитивы, измерять текст и переводить события системы в события uison.

В языке экранов нет кода. Текст и свойства привязаны к данным
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), элементы — переиспользуемые шаблоны
с параметрами, цвета и размеры берутся из токенов темы, а события вызывают именованные
действия, обработчики которых написаны на Rust, C или C++. Когда обработчик меняет данные,
экран перестраивается сам.

## Hello, world

Одно и то же окно — счётчик и кнопка «+1» — на трёх языках.

**Rust** — экран строится DSL, обработчик — замыкание у кнопки:

```rust
use uison::prelude::*;

fn main() {
    let screen = center(column().gap(12).cross("center").children([
        text("Pressed {@n} times").size(20),
        button("+1").key("inc").on_click(|cx| cx.data.edit("n", |n: &mut i32| *n += 1)),
    ]));
    let mut ui = Ui::from_node(screen).with_size(320.0, 200.0);
    ui.data.set("n", 0);
    run(ui, "Hello");
}
```

**C** — экран — JSON, записанный прямо в коде, обработчик — функция:

```c
#include <uison_kit.h>

static void inc(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [320, 200],
        data: { n: 0 },
        root: { Center: { child: { Column: { gap: 12, cross: "center", children: [
            { Text: { text: "Pressed {@n} times", size: 20 } },
            { Button: { key: "inc", text: "+1", on: { click: "inc" } } },
        ] } } } }
    }));
    if (!ui) return uison_kit_fail(); /* prints the JSON error and where it is */
    UISON_ON(ui, inc);                /* action "inc" → function inc */
    return uison_run(ui, "Hello");
}
```

**C++** — виджеты как во Flutter, обработчик — лямбда у кнопки:

```cpp
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Hello");
}
```

Чтобы начать свой проект, смотрите быстрый старт для
[Rust](../ru/rust/quickstart.md), [C](../ru/c/quickstart.md) или [C++](../ru/cpp/quickstart.md).

## Возможности

- **Без зависимостей.** Ядро, парсер JSON, раскладка, клиент Wayland, клиент D-Bus для
  портала настроек Linux и привязки COM для Windows написаны здесь же, поверх `std`.
  В `Cargo.lock` — только собственные крейты workspace.
- **Одна программа, шесть платформ.** macOS, Windows (x64 и ARM64), Linux (Wayland и X11,
  arm64 и x86_64), iOS, Android (arm64-v8a и armeabi-v7a) и браузер (WebAssembly). На телефоне
  тот же код получает касания, безопасную зону, экранную клавиатуру и прокрутку с инерцией.
- **Частичная перерисовка.** Кадры сравниваются, и перерисовываются только повреждённые
  области; тонкие линии выравниваются по физическим пикселям одинаково во всех бэкендах.
- **Библиотека элементов**: кнопки, кнопки-иконки и 25 иконок, флажки, переключатели-радио,
  выключатели, ползунки, выпадающие списки, вкладки, текстовые поля с выделением и буфером
  обмена, индикаторы прогресса, карточки, строки списков, значки, аватары, подсказки,
  контекстные меню, ссылки и картинки. Сами элементы — JSON-шаблоны (`lib/*.json`), и свои
  компоненты пишутся так же.
- **Раскладка в духе Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, прокрутка,
  виртуальные списки на десятки тысяч строк и `Flow` — абзац из строчных отрезков текста
  с переносом через их границы.
- **Привязки и состояние**: пути в данные (`@user.name`), интерполяция текста, локальное
  состояние компонента (`@~open`), селекторы по состоянию (hover, pressed, focus), `switch`
  и несколько функций (`eq`, `not`, `len`, `ratio`…). Структуры отображаются в данные и обратно:
  `#[derive(Data)]` в Rust, таблицы полей в C, рефлексия без макросов в C++20.
- **Темы**: светлая, тёмная и «как в системе», из токенов, которые можно переопределить.
- **Своя отрисовка**: узел `Canvas` вызывает вашу функцию, и она пишет примитивы в его
  прямоугольник — рядом с обычными элементами.
- **Потоки**: рабочий поток отправляет результаты обратно в поток интерфейса (`post`)
  на всех платформах, кроме браузера.
- **Живая перезагрузка** в отладочных сборках: с `UISON_RELOAD=folder` JSON-файлы экрана
  перечитываются при сохранении.
- **Проверка без окна**: сценарий (`UISON_SCRIPT=scenario.json`) нажимает, вводит текст,
  прокручивает и делает снимки по `key` элементов; одни и те же сценарии выполняются на всех
  платформах, в headless Chrome, в симуляторе iOS и на телефоне.
- **[Режим агента](../ru/agents.md)**: LLM-агент работает с программой без снимков экрана. Он читает
  экран по смыслу (роли, имена, состояния, ключи) и нажимает, вводит и выбирает тем же путём, что и
  человек, через JSON-строки (`--agent`) или как сервер MCP (`--mcp`).
- **API для C и C++**: `uison.h` (C99), `uison_kit.h` (помощники для C), `uison.hpp` (C++17,
  удобства C++20) и `uison_widgets.hpp` (виджеты C++20, сгенерированные из той же схемы, что
  и Rust DSL); статическая и динамическая библиотеки, пакет CMake и `pkg-config`.
  C и C++ собираются и для браузера через Emscripten: `em++ main.cpp libuison.a -o index.html`.

## Платформы

| платформа | бэкенд | где проверено |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO через рантайм Objective-C | Mac на Apple Silicon — основная машина разработки |
| iOS | UIKit, CADisplayLink, тот же код CoreGraphics/CoreText, что и на macOS | симулятор и iPhone 12 mini |
| Android | JNI вручную, `android.graphics.Canvas`, прослойка на Java ~300 строк; APK собирается без Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite и WIC, COM вручную; DPI для каждого монитора, тёмный заголовок | ПК с Windows 11 при 100/125/150 % и два монитора с разным масштабом |
| Linux | свой код протокола Wayland и X11 (xcb), Cairo + Pango, xkbcommon — всё через `dlopen`, поэтому бинарник зависит только от libc | Mutter, KWin, Sway, Weston и X11 без экрана (arm64 и x86_64); Ubuntu 24.04 с GNOME в виртуальной машине |
| Браузер | `wasm32-unknown-unknown` без wasm-bindgen, canvas 2D; C/C++ через Emscripten | Chrome (и headless Chrome) |

Пока не проверено: настоящий сенсорный экран на Windows, Windows on ARM (собирается, но не
запускалась), Linux на настоящем железе и KDE, ввод мышью и клавиатурой в GNOME, Safari и Firefox.

## Размер и скорость

Release-сборки (`opt-level = "s"`, LTO, `panic = "abort"`, без символов). Размеры — из
собственных замеров проекта по мере его роста, так что свежие сборки могут отличаться
на несколько процентов.

| программа | размер |
|---|---|
| урок 1 (счётчик выше), Rust, macOS | ~0,7 МБ |
| то же на C / C++ с `libuison.a`, macOS | 672 КБ / 816 КБ |
| Studio — главное демо (почта на 2000 писем, канбан, контакты, графики, формы), macOS | ~840 КБ |
| Studio, Windows x64 / ARM64 | 672 КБ / 613 КБ, Visual C++ Redistributable не нужен |
| Studio, Linux arm64 / x86_64 | 857 КБ / 936 КБ |
| Studio, Android `libmain.so` arm64 | ~820 КБ |
| Studio, браузер (`.wasm`, gzip) | ~220 КБ |

| замер | результат |
|---|---|
| событие → окно перерисовано, macOS | ~0,8 мс, из них ~0,2 мс — работа самого uison |
| касание → кадр, iPhone 12 mini | 3,6 мс; прокрутка списка — ровные 60 кадров/с |
| полная перерисовка Studio, iPhone 12 mini / Huawei P40 lite | 0,7 мс / 0,4–0,7 мс |
| событие мыши → кадр, Windows (Iris Xe, 60 Гц) | p50 1,4–1,7 мс |
| событие → кадр, Chrome | медиана 2,2 мс |
| запуск → первый кадр | iOS 37–41 мс, Android 58–68 мс, Windows 73–103 мс, браузер 24–42 мс |

[Disk Usage](../../examples/disk-usage/README.md), порт программы на egui, даёт прямое
сравнение: на macOS arm64 бинарник весит 857 КБ вместо 5,1 МБ, без зависимостей вместо
362 крейтов в `Cargo.lock`, с той же скоростью сканирования и тем же расходом памяти.

## Документация

Документация есть на русском и на английском. Ссылки ниже ведут на русскую версию, а на каждой
странице есть переключатель на [английскую](../README.md).

- [Выберите язык](../ru/README.md) — с чего начать.
- Серии уроков, каждая читается сама по себе, от подготовки системы до потоков и своего
  кадра: [Rust](../ru/rust/README.md), [C](../ru/c/README.md), [C++](../ru/cpp/README.md).
  Уроки ядра — одна и та же программа на трёх языках, и их снимки совпадают до пикселя.
- [Язык экранов](../ru/screens/README.md): [привязки и значения](../ru/screens/bindings.md),
  [раскладка](../ru/screens/layout.md), [элементы](../ru/screens/elements.md),
  [темы](../ru/screens/themes.md).
- [iOS и Android](../ru/mobile.md), [браузер (WebAssembly)](../ru/wasm.md).

## Примеры

| пример | что показывает |
|---|---|
| `studio` | главное демо: почта на 2000 писем, канбан, контакты, графики с анимацией, форма, настройки; телефонная раскладка меньше 700 pt |
| `gallery` | все элементы библиотеки |
| `disk-usage` | порт приложения на egui: солнечная диаграмма в узле `Canvas`, панели и меню — экран uison, параллельный сканер на `std` |
| `browser` | маленький веб-браузер: свои парсеры HTML и CSS и каскад, HTTP и HTTPS через TLS системы; страница становится экраном uison |
| `todo` | список дел на Rust DSL |
| `list10k` | виртуальный список на 10 000 строк |
| `form2`, `settings` | формы и экраны настроек |
| `demo`, `scroll`, `uibench` | примитивы вручную, прокрутка, бенчмарк |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Примеры на C и C++ лежат в [`examples-c`](../../examples-c) и [`examples-cpp`](../../examples-cpp),
у каждой папки свой CMake-проект.

## Сборка и проверка

Нужен stable Rust; больше ничего на macOS, Windows (с тулчейном MSVC) и Linux
(без `-dev`-пакетов: системные библиотеки загружаются во время работы). Подготовка каждой
системы — урок 0 каждой серии ([Rust](../ru/rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

На Windows скрипты — `tools\*.ps1`: `tools\win.ps1 studio` собирает пример, прогоняет сценарий
и сохраняет снимки, `tools\c-dist.ps1 -Test` собирает и проверяет пакет C/C++.

Переключатели бэкенда для отладки, везде одинаковые: `UISON_STATS=1` печатает тайминги
событий, кадров и анимации, `UISON_FLASH=1` подсвечивает перерисованные области,
а `UISON_SCRIPT` / `UISON_CAPTURE` выполняют сценарий и сохраняют снимки.

## Структура

```
crates/
  uison-json      pull-парсер JSON и дерево значений: комментарии, висячие запятые, ключи без кавычек
  uison-core      геометрия, цвета, шрифты, события, трейты Canvas и Host, трейт App
  uison-frame     кадр примитивов: разбор, diff → повреждения, hit-test, исполнитель сценариев
  uison-ui        компилятор экранов: шаблоны, привязки, темы, раскладка, рантайм, Rust DSL, lib/*.json
  uison-apple     общее для macOS и iOS: FFI Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     бэкенд AppKit
  uison-ios       бэкенд UIKit
  uison-android   бэкенд на JNI и его прослойка на Java (java/dev/uison)
  uison-windows   бэкенд Win32 + Direct2D + DirectWrite
  uison-linux     бэкенд Wayland и X11, Cairo + Pango, всё через dlopen
  uison-web       бэкенд WebAssembly (crates/uison-web/uison.js)
  uison-c         C API и заголовки C++, статическая и динамическая библиотеки, пакет CMake
  uison-gen       генерирует виджеты C++, Rust DSL и справочник элементов из схемы экранов
  uison-derive    #[derive(Data)] без syn
  uison           зонтичный крейт: всё для текущей платформы, uison::prelude; уроки Rust
examples/         примеры на Rust (см. выше)
examples-c/       примеры и уроки на C, CMake-проект
examples-cpp/     примеры и уроки на C++, CMake-проект
scenarios/        сценарии проверки, общие для всех платформ
tools/            скрипты сборки, запуска и проверки для каждой платформы; драйверы настоящего ввода мышью и клавиатурой
web/              страница и JavaScript-часть бэкенда WebAssembly
docs/             документация на английском; ru/ — то же на русском; readme/ — этот README на других языках
```

## Ограничения

- Одно окно на программу; перетаскивания (drag-and-drop) нет.
- Ввод текста поддерживает латиницу и кириллицу и экранные клавиатуры телефонов; составной
  ввод для китайского, японского и корейского (IME) на компьютере не сделан, как и текст
  справа налево.
- Поддержки специальных возможностей (экранных дикторов) пока нет.
- В браузере нет потоков.
- Фреймворк молодой: версия 0.1, API может измениться.

## Переводы

Документация написана на английском, а в `docs/ru/` лежат те же страницы на русском,
с переключателем языка вверху каждой. Этот README тоже переведён:
`docs/readme/README.<code>.md`, по одному на язык, со списком языков вверху каждого.
Изменение здесь нужно внести и в переводы.

## Лицензия

Лицензия пока не выбрана.
