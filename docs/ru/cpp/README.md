# uison на C++

[English](../../cpp/README.md) · **Русский**

Экран — виджеты C++20, как во Flutter (`Column({.gap = 12, .children = {Text("…"), Button("+1", {.on_click = …})}})`), логика — лямбды прямо у элементов, данные — ваши структуры; окно, отрисовка и ввод — uison. Одна программа работает на macOS, Windows, Linux (Wayland и X11), iOS, Android и в браузере. Подключение — статическая или динамическая библиотека, заголовки `uison.hpp` (C++17) и `uison_widgets.hpp` (C++20).

- **[Быстрый старт](quickstart.md)** — первая программа и сборка одной командой или CMake.
- **[Язык экранов](../screens/README.md)** — общий справочник: привязки, раскладка, [элементы](../screens/elements.md), темы.
- **[Телефоны](../mobile.md)** и **[браузер](../wasm.md)**.
- **Справочник API** — комментарии в [`uison.hpp`](../../../crates/uison-c/include/uison.hpp) и [`uison_widgets.hpp`](../../../crates/uison-c/include/uison_widgets.hpp) (генерируется `uison-gen`; подсказки IDE показывают описания полей).
- **Примеры** — [`examples-cpp/`](../../../examples-cpp): `todo-cpp`, `list10k-cpp`, `worker-cpp`, тест `cpp-test`; `cmake -S examples-cpp -B out/cpp/build && cmake --build out/cpp/build`.

## Уроки

Каждый урок — программа в [`examples-cpp/lessons`](../../../examples-cpp/lessons). `tools/lessons.sh cpp` собирает их все, прогоняет сценарии и обновляет снимки. Уроки со звёздочкой — та же программа, что в сериях [Rust](../rust/README.md) и [C](../c/README.md): снимки трёх языков совпадают до пикселя (`tools/lessons-compare.sh`).

| | урок | о чём |
|---|---|---|
| 0 | [Подготовка системы](lessons/00-setup.md) | компилятор C++20, CMake, Rust или готовый пакет; macOS, Linux, Windows и Visual Studio |
| 1 ★ | [Окно и кнопка](lessons/01-hello.md) | `Column`, `Text`, `Button`, лямбда у кнопки, `uison::Screen` |
| 2 ★ | [Данные и привязки](lessons/02-data.md) | привязки в виджетах, `For`, `derive`, `get`/`set`/`update` |
| 3 ★ | [Форма с проверкой](lessons/03-forms.md) | форма как структура, ошибки полей, фокус |
| 4 ★ | [Списки](lessons/04-lists.md) | `List` на 10 000 строк, `key`, `vector<Person>` в данные |
| 5 | [Данные как структуры](lessons/05-structs.md) | рефлексия агрегатов: вложенные, `optional`, `map`; состояние экрана структурой |
| 6 | [Свои компоненты](lessons/06-components.md) | `components.json` + виджеты из генератора (`uison_generate_widgets`) |
| 7 ★ | [Темы и токены](lessons/07-themes.md) | светлая, тёмная, системная; цвет акцента на лету |
| 8 ★ | [Картинки, подсказки, меню](lessons/08-rich.md) | `Image`, `Avatar`, `tooltip`, `context_menu`, `Link`, буфер обмена |
| 9 ★ | [Потоки](lessons/09-threads.md) | `std::jthread`, `Handle::post`, отмена через `stop_token` |
| 10 ★ | [Свой кадр](lessons/10-frame.md) | `uison::App`, `FrameWriter`: примитивы и события сами |
| 11 | [C++17 и JSON](lessons/11-cpp17.md) | цепочка методов `Node`, экран JSON-строкой и файлом: когда что |
| 12 | [Сборка, телефоны, браузер](lessons/12-build.md) | CMake из исходников и пакетом, статическая или динамическая, iOS, Android, браузер |
