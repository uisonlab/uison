# Урок 1. Окно и кнопка

[English](../../../c/lessons/01-hello.md) · **Русский**

**Что будет:** окно с надписью «Pressed N times» и кнопкой «+1».

![Урок 1](../../../c/img/01-hello.png)

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

```sh
cmake --build out/c/build --target c-01-hello && ./out/c/build/c-01-hello
```

## Как это работает

1. **Экран — JSON прямо в коде.** `UISON_JSON({ … })` из `uison_kit.h` превращает текст в строку препроцессором: кавычки и `\` экранирует он сам, поэтому JSON пишется как есть. Расширенный JSON: ключи без кавычек, висячие запятые, комментарии `//`. `data` — данные приложения (здесь одно число `n`), `root` — дерево элементов: `Center` ставит ребёнка по центру, `Column` выстраивает детей столбиком, `Text` и `Button` — элементы библиотеки ([справочник](../../screens/elements.md)).
2. **Привязки.** `"Pressed {@n} times"` — подставить значение `n`. Когда данные меняются, экран пересобирается сам: обновлять текст руками не нужно ([привязки](../../screens/bindings.md)).
3. **Действия.** `on: { click: "inc" }` — при нажатии вызвать действие `inc`. Логики в JSON нет: обработчик — функция в программе. `UISON_ON(ui, inc)` — то же, что `uison_ui_on(ui, "inc", inc, NULL, NULL)`: имя действия — имя функции.
4. **Данные в обработчике.** `uison_cx_data(cx)` — данные приложения; к значениям — по путям: `"n"`, `"form.email"`, `"todos.3.done"`.
5. **Ошибка в JSON.** `uison_ui_new` вернёт `NULL`, а `uison_kit_fail()` напечатает ошибку и вернёт 1. JSON из `UISON_JSON` — одна строка, поэтому в ошибке есть фрагмент вокруг места: `expected ':' near "…{ Text: { text <here> \"Pressed…"`.
6. **Запуск.** `uison_run` открывает окно и забирает `ui` (освобождать не нужно).

Аргументы `user` и `drop` у `uison_ui_on` — свои данные обработчика: `user` приходит в каждый вызов, `drop(user)` библиотека вызовет, когда обработчик больше не нужен. Здесь не нужны — `NULL`; с ними — `UISON_ON_USER(ui, f, user, drop)` ([урок 8](08-threads.md)).

**Ограничения `UISON_JSON`.** Круглые скобки вне строк должны быть парными (в экранах их вне строк нет). Строка `#` в начале строки кода — директива препроцессора, но цвета всегда в кавычках, и это не мешает. Стандарт C99 обещает строки до 4095 символов: экран больше (`-pedantic` предупреждает) — в файл, [урок 5](05-files.md).

## Попробуйте

- Добавьте кнопку «−1» с действием `dec`.
- Сделайте ошибку в JSON (`txt` вместо `text`) и посмотрите, что напечатает `uison_kit_fail`.

Дальше — [урок 2: данные и привязки](02-data.md).
