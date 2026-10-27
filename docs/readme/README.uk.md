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
<a href="README.ru.md">🇷🇺 Русский</a> ·
<a href="README.ur.md">🇵🇰 اردو</a> ·
<a href="README.id.md">🇮🇩 Bahasa Indonesia</a> ·
<a href="README.de.md">🇩🇪 Deutsch</a> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<b>🇺🇦 Українська</b>
</h3>
<!-- /languages -->

**uison** (UI + JSON) — маленький швидкий UI-фреймворк на Rust зовсім без залежностей:
лише стандартна бібліотека, аж до власного парсера JSON. Екран описується деревом
елементів: у JSON-файлі, на Rust DSL, віджетами C++ у дусі Flutter або JSON просто в коді
на C. Логіка — звичайний код на Rust, C або C++: обробники, дані, потоки. Вікно, розкладку,
малювання та введення бере на себе uison, і та сама програма працює на macOS, Windows,
Linux (Wayland і X11), iOS, Android і в браузері.

Кожен бекенд звертається до системи напряму, через FFI, написаний вручну: AppKit і CoreText,
Win32 з Direct2D і DirectWrite (COM вручну), протокол Wayland, реалізований з нуля, і X11,
UIKit, Canvas Android через JNI та canvas 2D з WebAssembly. Release-програма — один файл
приблизно 0,7–0,9 МБ, якому потрібні лише бібліотеки самої системи, а подія доходить до
екрана на комп'ютері приблизно за мілісекунду.

![Disk Usage, приклад на uison: сонячна діаграма теки, список файлів і підказка](../img/disk-usage.jpg)

## Як це влаштовано

```text
екран (JSON, Rust DSL, віджети C++, UISON_JSON у C)  +  дані  +  обробники вашою мовою
        │  прив'язки, шаблони, теми, розкладка (uison-ui)
        ▼
кадр примітивів: rect, line, circle, tri, poly, text, image, clip, hit
        │  порівняння з попереднім кадром → перемалювати лише змінене
        ▼
бекенд системи: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Екран компілюється в *кадр* — плаский список примітивів кількох видів: прямокутники, лінії,
кола, багатокутники, однорядковий текст, зображення, області обрізання та невидимі області
для миші й дотиків. Кожен новий кадр порівнюється з попереднім, і бекенд перемальовує лише
змінені області. Бекенд виходить маленьким: йому потрібно лише малювати ці примітиви,
вимірювати текст і перетворювати події системи на події uison.

У мові екранів немає коду. Текст і властивості прив'язані до даних
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), елементи — шаблони з параметрами,
які можна використовувати повторно, кольори й розміри беруться з токенів теми, а події
викликають іменовані дії, обробники яких написані на Rust, C або C++. Коли обробник змінює
дані, екран перебудовується сам.

## Hello, world

Те саме вікно — лічильник і кнопка «+1» — трьома мовами.

**Rust** — екран будується DSL, обробник — замикання біля кнопки:

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

**C** — екран — це JSON, записаний просто в коді, обробник — функція:

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

**C++** — віджети як у Flutter, обробник — лямбда біля кнопки:

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

Щоб почати власний проєкт, див. швидкий старт для
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) або [C++](../cpp/quickstart.md).

## Можливості

- **Без залежностей.** Ядро, парсер JSON, розкладка, клієнт Wayland, клієнт D-Bus для
  порталу налаштувань Linux і прив'язки COM для Windows написані тут само, поверх `std`.
  У `Cargo.lock` — лише власні крейти workspace.
- **Одна програма, шість платформ.** macOS, Windows (x64 і ARM64), Linux (Wayland і X11,
  arm64 і x86_64), iOS, Android (arm64-v8a і armeabi-v7a) і браузер (WebAssembly). На
  телефоні той самий код отримує дотики, безпечну зону, екранну клавіатуру й прокручування
  з інерцією.
- **Часткове перемальовування.** Кадри порівнюються, і перемальовуються лише пошкоджені
  області; тонкі лінії вирівнюються за фізичними пікселями однаково в усіх бекендах.
- **Бібліотека елементів**: кнопки, кнопки-іконки та 25 іконок, прапорці, радіокнопки,
  перемикачі, повзунки, випадні списки, вкладки, текстові поля з виділенням і буфером обміну,
  індикатори прогресу, картки, рядки списків, значки, аватари, підказки, контекстні меню,
  посилання та зображення. Самі елементи — JSON-шаблони (`lib/*.json`), і власні компоненти
  пишуться так само.
- **Розкладка в дусі Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex,
  прокручування, віртуалізовані списки на десятки тисяч рядків і `Flow` — абзац із рядкових
  фрагментів тексту з перенесенням через їхні межі.
- **Прив'язки та стан**: шляхи в дані (`@user.name`), інтерполяція тексту, локальний стан
  компонента (`@~open`), селектори за станом (hover, pressed, focus), `switch` і кілька
  функцій (`eq`, `not`, `len`, `ratio`…). Структури відображаються в дані й назад:
  `#[derive(Data)]` у Rust, таблиці полів у C, рефлексія без макросів у C++20.
- **Теми**: світла, темна й «як у системі», з токенів, які можна перевизначити.
- **Власне малювання**: вузол `Canvas` викликає вашу функцію, і вона пише примітиви в його
  прямокутник — поряд зі звичайними елементами.
- **Потоки**: робочий потік надсилає результати назад у потік інтерфейсу (`post`) на всіх
  платформах, крім браузера.
- **Живе перезавантаження** у налагоджувальних збірках: з `UISON_RELOAD=folder` JSON-файли
  екрана перечитуються, коли ви їх зберігаєте.
- **Тестування без вікна**: сценарій (`UISON_SCRIPT=scenario.json`) натискає, вводить текст,
  прокручує й робить знімки за `key` елементів; ті самі сценарії виконуються на всіх
  платформах, у headless Chrome, у симуляторі iOS і на телефоні.
- **API для C і C++**: `uison.h` (C99), `uison_kit.h` (помічники для C), `uison.hpp` (C++17,
  зручності C++20) і `uison_widgets.hpp` (віджети C++20, згенеровані з тієї самої схеми, що
  й Rust DSL); статична й динамічна бібліотеки, пакет CMake і `pkg-config`.
  C і C++ також збираються для браузера через Emscripten: `em++ main.cpp libuison.a -o index.html`.

## Платформи

| платформа | бекенд | де перевірено |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO через рантайм Objective-C | Mac на Apple Silicon — основна машина розробки |
| iOS | UIKit, CADisplayLink, той самий код CoreGraphics/CoreText, що й на macOS | симулятор та iPhone 12 mini |
| Android | JNI вручну, `android.graphics.Canvas`, прошарок на Java ~300 рядків; APK збирається без Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite і WIC, COM вручну; DPI для кожного монітора, темний заголовок | ПК з Windows 11 при 100/125/150 % і два монітори з різним масштабом |
| Linux | власний код протоколу Wayland і X11 (xcb), Cairo + Pango, xkbcommon — усе через `dlopen`, тож бінарник залежить лише від libc | Mutter, KWin, Sway, Weston і X11 без екрана (arm64 і x86_64); Ubuntu 24.04 з GNOME у віртуальній машині |
| Браузер | `wasm32-unknown-unknown` без wasm-bindgen, canvas 2D; C/C++ через Emscripten | Chrome (і headless Chrome) |

Ще не перевірено: справжній сенсорний екран у Windows, Windows on ARM (збирається, але не
запускалася), Linux на справжньому залізі та KDE, введення мишею й клавіатурою в GNOME,
Safari і Firefox.

## Розмір і швидкість

Release-збірки (`opt-level = "s"`, LTO, `panic = "abort"`, без символів). Розміри — з власних
вимірювань проєкту в міру його зростання, тож найновіші збірки можуть відрізнятися на кілька
відсотків.

| програма | розмір |
|---|---|
| урок 1 (лічильник вище), Rust, macOS | ~0,7 МБ |
| те саме на C / C++ з `libuison.a`, macOS | 672 КБ / 816 КБ |
| Studio — головне демо (пошта на 2000 листів, канбан, контакти, графіки, форми), macOS | ~840 КБ |
| Studio, Windows x64 / ARM64 | 672 КБ / 613 КБ, Visual C++ Redistributable не потрібен |
| Studio, Linux arm64 / x86_64 | 857 КБ / 936 КБ |
| Studio, Android `libmain.so` arm64 | ~820 КБ |
| Studio, браузер (`.wasm`, gzip) | ~220 КБ |

| вимірювання | результат |
|---|---|
| подія → вікно перемальоване, macOS | ~0,8 мс, з них ~0,2 мс — робота самого uison |
| дотик → кадр, iPhone 12 mini | 3,6 мс; прокручування списку — рівні 60 кадрів/с |
| повне перемальовування Studio, iPhone 12 mini / Huawei P40 lite | 0,7 мс / 0,4–0,7 мс |
| подія миші → кадр, Windows (Iris Xe, 60 Гц) | p50 1,4–1,7 мс |
| подія → кадр, Chrome | медіана 2,2 мс |
| запуск → перший кадр | iOS 37–41 мс, Android 58–68 мс, Windows 73–103 мс, браузер 24–42 мс |

[Disk Usage](../../examples/disk-usage/README.md), порт програми на egui, дає пряме
порівняння: на macOS arm64 бінарник важить 857 КБ замість 5,1 МБ, без залежностей замість
362 крейтів у `Cargo.lock`, з тією самою швидкістю сканування й тим самим споживанням пам'яті.

## Документація

Документація написана англійською, і кожна сторінка має [російську версію](../ru/README.md).

- [Оберіть мову](../README.md) — з чого почати.
- Серії уроків, кожну можна читати окремо, від підготовки системи до потоків і власного
  кадру: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Базові уроки — та сама програма трьома мовами, і їхні знімки збігаються до пікселя.
- [Мова екранів](../screens/README.md): [прив'язки та значення](../screens/bindings.md),
  [розкладка](../screens/layout.md), [елементи](../screens/elements.md),
  [теми](../screens/themes.md).
- [iOS та Android](../mobile.md), [браузер (WebAssembly)](../wasm.md).

## Приклади

| приклад | що показує |
|---|---|
| `studio` | головне демо: пошта на 2000 листів, канбан, контакти, анімовані графіки, форма, налаштування; телефонна розкладка для ширини менше 700 pt |
| `gallery` | усі елементи бібліотеки |
| `disk-usage` | порт застосунку на egui: сонячна діаграма у вузлі `Canvas`, панелі й меню — екран uison, паралельний сканер на `std` |
| `browser` | маленький веббраузер: власні парсери HTML і CSS та каскад, HTTP і HTTPS через TLS системи; сторінка стає екраном uison |
| `todo` | список справ на Rust DSL |
| `list10k` | віртуалізований список на 10 000 рядків |
| `form2`, `settings` | форми й екрани налаштувань |
| `demo`, `scroll`, `uibench` | примітиви вручну, прокручування, бенчмарк |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Приклади на C і C++ лежать у [`examples-c`](../../examples-c) та [`examples-cpp`](../../examples-cpp),
кожна тека — окремий проєкт CMake.

## Збирання та тестування

Потрібен stable Rust; більше нічого на macOS, Windows (з тулчейном MSVC) і Linux
(без пакетів `-dev`: системні бібліотеки завантажуються під час роботи). Підготовка кожної
системи — урок 0 кожної серії ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

У Windows скрипти — `tools\*.ps1`: `tools\win.ps1 studio` збирає приклад, виконує сценарій
і зберігає знімки, `tools\c-dist.ps1 -Test` збирає й тестує пакет C/C++.

Перемикачі бекенда для налагодження, скрізь однакові: `UISON_STATS=1` виводить час обробки
подій, кадрів і анімації, `UISON_FLASH=1` підсвічує перемальовані області, а
`UISON_SCRIPT` / `UISON_CAPTURE` виконують сценарій і зберігають знімки.

## Структура

```
crates/
  uison-json      pull-парсер JSON і дерево значень: коментарі, кінцеві коми, ключі без лапок
  uison-core      геометрія, кольори, шрифти, події, трейти Canvas і Host, трейт App
  uison-frame     кадр примітивів: розбір, diff → пошкодження, hit-test, виконавець сценаріїв
  uison-ui        компілятор екранів: шаблони, прив'язки, теми, розкладка, рантайм, Rust DSL, lib/*.json
  uison-apple     спільне для macOS та iOS: FFI Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     бекенд AppKit
  uison-ios       бекенд UIKit
  uison-android   бекенд на JNI та його прошарок на Java (java/dev/uison)
  uison-windows   бекенд Win32 + Direct2D + DirectWrite
  uison-linux     бекенд Wayland і X11, Cairo + Pango, усе через dlopen
  uison-web       бекенд WebAssembly (web/uison.js)
  uison-c         C API і заголовки C++, статична й динамічна бібліотеки, пакет CMake
  uison-gen       генерує віджети C++, Rust DSL і довідник елементів зі схеми екранів
  uison-derive    #[derive(Data)] без syn
  uison           парасольковий крейт: усе для поточної платформи, uison::prelude; уроки Rust
examples/         приклади на Rust (див. вище)
examples-c/       приклади й уроки на C, проєкт CMake
examples-cpp/     приклади й уроки на C++, проєкт CMake
scenarios/        тестові сценарії, спільні для всіх платформ
tools/            скрипти збирання, запуску й тестування для кожної платформи; драйвери справжнього введення мишею й клавіатурою
web/              сторінка й JavaScript-частина бекенда WebAssembly
docs/             документація англійською; ru/ — те саме російською; readme/ — цей README іншими мовами
```

## Обмеження

- Одне вікно на програму; перетягування (drag-and-drop) немає.
- Введення тексту підтримує латиницю, кирилицю та екранні клавіатури телефонів; складене
  введення для китайської, японської та корейської (IME) на комп'ютері не зроблене, як і
  текст справа наліво.
- Підтримки спеціальних можливостей (екранних читачів) поки немає.
- У браузері немає потоків.
- Фреймворк молодий: версія 0.1, API може змінитися.

## Переклади

Документація написана англійською, а в `docs/ru/` лежать ті самі сторінки російською,
з перемикачем мови вгорі кожної. Цей README теж перекладено:
`docs/readme/README.<code>.md`, по одному на мову, зі списком мов угорі кожного.
Зміну тут треба внести й у переклади.

## Ліцензія

Ліцензію ще не обрано.
