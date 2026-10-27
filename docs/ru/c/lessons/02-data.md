# Урок 2. Данные и привязки

[English](../../../c/lessons/02-data.md) · **Русский**

**Что будет:** поле «Name» и приветствие, которое меняется по мере ввода; переключатель, который его прячет; теги-значки и поле, которое добавляет новые.

![Урок 2](../../../c/img/02-data.png)

<!-- code: examples-c/lessons/02-data/main.c -->
```c
/* Lesson 2: data and bindings. Elements read data by paths (@name), an input field writes it;
   code changes the data, and the screen follows it by itself. */
#include <uison_kit.h>

/* Derived data: derive is called after any change. */
static void count(uison_data* d, void* user) {
    (void)user;
    size_t n = uison_data_len(d, "tags");
    uison_data_set_f64(d, "count", (double)n);
    uison_data_set_bool(d, "many", n >= 5);
}

/* A new tag goes to the end of the array: %s escapes the string itself (quotes, \, line breaks). */
static void add(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_pushf(d, "tags", "%s", uison_data_str(d, "draft"));
    uison_data_set_str(d, "draft", "");
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [420, 360],
        data: { name: "World", show: true, tags: ["rust", "c++"], draft: "" },
        root: { Column: { padding: 20, gap: 12, cross: "stretch", children: [
            // Two-way binding: the field writes to name, the text below reads it.
            { TextField: { key: "name", label: "Name", value: "@name" } },
            { Text: { text: "Hello, {@name}!", size: 20, bold: true, visible: "@show" } },
            { Switch: { key: "show", label: "Show greeting", checked: "@show" } },
            // A list: For repeats the template for each element of the tags array.
            { Wrap: { gap: 6, children: [ { For: { each: "@tags", as: "t", do: { Badge: { text: "@t" } } } } ] } },
            { Row: { gap: 8, children: [
                { TextField: { key: "draft", value: "@draft", placeholder: "new tag", flex: 1, on: { submit: "add" } } },
                { Button: { key: "add", text: "Add", disabled: "!@draft", on: { click: "add" } } },
            ] } },
            // A conditional value: a switch on data.
            { Text: { text: "Tags: {@count}", color: { switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" } } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    uison_ui_derive(ui, count, NULL, NULL);
    UISON_ON(ui, add);
    return uison_run(ui, "Lesson 2 — data");
}
```

## Как это работает

1. **Двусторонняя привязка.** `TextField { value: "@name" }` показывает `name` и пишет в него при вводе. `Text { text: "Hello, {@name}!" }` читает то же значение — приветствие меняется на каждую букву, без обработчиков.
2. **Видимость.** `visible: "@show"` — узел есть, пока `show` истинно; `"!@show"` — наоборот. Ложь — `false`, `0`, `""`, `[]`, `null`.
3. **Списки.** `For { each: "@tags", as: "t", do: … }` повторяет шаблон для каждого элемента массива `tags`; внутри `@t` — текущий элемент.
4. **Данные с подстановкой.** `uison_data_pushf(d, "tags", "%s", text)` добавляет в конец массива значение, собранное по формату: `%s` — строка в кавычках, с экранированными `"`, `\` и переводами строк. Формат — JSON: `uison_data_pushf(d, "todos", "{ id: %d, title: %s, done: %s }", id, t, UISON_B(done))`. Компилятор проверяет типы аргументов, как у `printf`.
5. **Производные данные.** `uison_ui_derive(ui, count, …)` вызывается после каждого изменения данных: здесь он считает теги. Экран привязан к результату (`{@count}`), поэтому счётчик всегда верный.
6. **Выбор значения.** `color: { switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" }` — цвет по данным. Селекторы и функции (`eq`, `not`, `len`…) — в [справочнике привязок](../../screens/bindings.md).

**Подстановка** (`uison_data_setf`, `uison_data_pushf`, `uison_fmt`):

| | что пишет |
|---|---|
| `%s` | строку в кавычках, экранированную; `NULL` — `null`; `UISON_B(x)` — `true` / `false` |
| `%d %u %ld %lld %zu …` | целое |
| `%g %f %e` | число без локали (в русской — тоже точка); `%g` без точности — кратчайшая запись; NaN — `null` |
| `%c`, `%%` | символ строкой, знак процента |

`null` пишите прямо в формате: GCC предупреждает о буквальном `NULL` у `%s`.

## Попробуйте

- Не добавлять пустой тег и тег, который уже есть.
- Удаление по нажатию на значок: `key: "tag-{@t}"`, обработчик читает `uison_cx_key`.

Дальше — [урок 3: форма с проверкой](03-forms.md).
