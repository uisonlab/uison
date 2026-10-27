# uison на C

[English](../../c/README.md) · **Русский**

Экран — JSON прямо в коде (`UISON_JSON({ … })`, без экранирования кавычек) или в файле, логика — функции на C, окно, отрисовка и ввод — uison. Одна программа работает на macOS, Windows, Linux (Wayland и X11), iOS, Android и в браузере. Подключение — статическая или динамическая библиотека с заголовком `uison.h` (C99); помощники — `uison_kit.h` (только заголовок).

- **[Быстрый старт](quickstart.md)** — первая программа, три способа сборки, статическая или динамическая.
- **[Язык экранов](../screens/README.md)** — общий справочник: привязки, раскладка, [элементы](../screens/elements.md), темы.
- **[Телефоны](../mobile.md)** и **[браузер](../wasm.md)**.
- **Справочник API** — комментарии в [`uison.h`](../../../crates/uison-c/include/uison.h) и [`uison_kit.h`](../../../crates/uison-c/include/uison_kit.h).
- **Примеры** — [`examples-c/`](../../../examples-c): `form2-c`, `app-c`, тесты `abi-test` и `kit-test`; `cmake -S examples-c -B out/c/build && cmake --build out/c/build`.

## Уроки

Каждый урок — программа в [`examples-c/lessons`](../../../examples-c/lessons). `tools/lessons.sh c` собирает их все, прогоняет сценарии и обновляет снимки. Уроки со звёздочкой — та же программа, что в сериях [Rust](../rust/README.md) и [C++](../cpp/README.md): снимки трёх языков совпадают до пикселя (`tools/lessons-compare.sh`).

| | урок | о чём |
|---|---|---|
| 0 | [Подготовка системы](lessons/00-setup.md) | компилятор, CMake, Rust или готовый пакет; macOS, Linux, Windows и Visual Studio |
| 1 ★ | [Окно и кнопка](lessons/01-hello.md) | `UISON_JSON`, действие и функция-обработчик, `uison_run`, ошибка в JSON и её место |
| 2 ★ | [Данные и привязки](lessons/02-data.md) | `@path`, `{@path}`, поле ввода, `For`, `derive`; `uison_data_pushf` |
| 3 ★ | [Форма с проверкой](lessons/03-forms.md) | ошибки полей, фокус; форма как структура — таблица полей |
| 4 ★ | [Списки](lessons/04-lists.md) | `List` на 10 000 строк, `key` строки; массив структур в данные одним вызовом |
| 5 | [Экран из файлов](lessons/05-files.md) | `uison_ui_load`, свои компоненты и `$param`, `UISON_RELOAD` |
| 6 ★ | [Темы и токены](lessons/06-themes.md) | светлая, тёмная, системная; цвет акцента на лету |
| 7 ★ | [Картинки, подсказки, меню](lessons/07-rich.md) | `Image`, `Avatar`, `tooltip`, `contextMenu`, `Link`, буфер обмена |
| 8 ★ | [Потоки](lessons/08-threads.md) | рабочий поток (pthread / `_beginthreadex`), `uison_post`, прогресс, отмена |
| 9 ★ | [Свой кадр](lessons/09-frame.md) | `uison_app`: примитивы и события сами |
| 10 | [Везде](lessons/10-everywhere.md) | та же программа на iPhone, Android и в браузере |
