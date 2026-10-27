# uison на Rust

[English](../../rust/README.md) · **Русский**

Экран — Rust DSL (`column().gap(12).children([…])`) или JSON-файл, логика — замыкания прямо у элементов, окно, отрисовка и ввод — uison. Одна программа работает на macOS, Windows, Linux (Wayland и X11), iOS, Android и в браузере. Зависимостей нет — только std.

- **[Быстрый старт](quickstart.md)** — свой крейт с uison за пять команд.
- **[Язык экранов](../screens/README.md)** — общий справочник: привязки, раскладка, [элементы](../screens/elements.md), темы.
- **[Телефоны](../mobile.md)** и **[браузер](../wasm.md)**.

## Уроки

Каждый урок — пример крейта `uison` в [`crates/uison/examples`](../../../crates/uison/examples): `cargo run --release -p uison --example 01-hello`. Скрипт `tools/lessons.sh rust` собирает их все, прогоняет сценарии и обновляет снимки. Уроки со звёздочкой — та же программа, что в сериях [C](../c/README.md) и [C++](../cpp/README.md): снимки трёх языков совпадают до пикселя (`tools/lessons-compare.sh`).

| | урок | о чём |
|---|---|---|
| 0 | [Подготовка системы](lessons/00-setup.md) | rustup, линкер системы, редактор: macOS, Linux, Windows |
| 1 ★ | [Окно и кнопка](lessons/01-hello.md) | DSL, замыкание у кнопки, `Ui`, `run` |
| 2 ★ | [Данные и привязки](lessons/02-data.md) | `@path`, `{@path}`, `bind`, `for_each`, `derive` |
| 3 ★ | [Форма с проверкой](lessons/03-forms.md) | форма как структура (`#[derive(Data)]`), ошибки, фокус |
| 4 ★ | [Списки](lessons/04-lists.md) | `list` на 10 000 строк, `key` строки, `Vec<Person>` в данные |
| 5 | [Экран из JSON](lessons/05-json.md) | `source!`, `Ui::load`, свои компоненты, `UISON_RELOAD`; DSL или JSON |
| 6 ★ | [Темы и токены](lessons/06-themes.md) | светлая, тёмная, системная; цвет акцента на лету |
| 7 ★ | [Картинки, подсказки, меню](lessons/07-rich.md) | `image`, `avatar`, `tooltip`, `context_menu`, `link`, буфер обмена |
| 8 ★ | [Потоки](lessons/08-threads.md) | `ui.handle()`, `Handle::post`, `std::thread`, отмена |
| 9 | [Тесты экрана](lessons/09-tests.md) | `cargo test` без окна: `tap`, `type_text`, `texts`; снимок `Headless` |
| 10 ★ | [Свой кадр](lessons/10-frame.md) | трейт `App`, `Writer`: примитивы и события сами |
| 11 | [Везде](lessons/11-everywhere.md) | iOS, Android, браузер для своего крейта |
