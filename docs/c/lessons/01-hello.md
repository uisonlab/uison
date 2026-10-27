# Lesson 1. A window and a button

**English** · [Русский](../../ru/c/lessons/01-hello.md)

**What you'll get:** a window with the label "Pressed N times" and a "+1" button.

![Lesson 1](../img/01-hello.png)

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

## How it works

1. **The screen is JSON right in your code.** `UISON_JSON({ … })` from `uison_kit.h` turns the text into a string with the preprocessor: it escapes quotes and `\` for you, so you write the JSON as is. It's extended JSON: unquoted keys, trailing commas, `//` comments. `data` is the application data (here a single number `n`), `root` is the element tree: `Center` centers its child, `Column` stacks its children vertically, `Text` and `Button` are library elements ([reference](../../screens/elements.md)).
2. **Bindings.** `"Pressed {@n} times"` substitutes the value of `n`. When the data changes, the screen rebuilds itself: you don't update the text by hand ([bindings](../../screens/bindings.md)).
3. **Actions.** `on: { click: "inc" }` — on press, invoke the `inc` action. There's no logic in JSON: the handler is a function in your program. `UISON_ON(ui, inc)` is the same as `uison_ui_on(ui, "inc", inc, NULL, NULL)`: the action name is the function name.
4. **Data in a handler.** `uison_cx_data(cx)` is the application data; you reach values by paths: `"n"`, `"form.email"`, `"todos.3.done"`.
5. **An error in JSON.** `uison_ui_new` returns `NULL`, and `uison_kit_fail()` prints the error and returns 1. JSON from `UISON_JSON` is a single line, so the error shows the fragment around the spot: `expected ':' near "…{ Text: { text <here> \"Pressed…"`.
6. **Running.** `uison_run` opens the window and takes ownership of `ui` (you don't free it).

The `user` and `drop` arguments of `uison_ui_on` are the handler's own data: `user` is passed to every call, and the library calls `drop(user)` when the handler is no longer needed. You don't need them here — `NULL`; with them, use `UISON_ON_USER(ui, f, user, drop)` ([lesson 8](08-threads.md)).

**`UISON_JSON` limitations.** Parentheses outside strings must be balanced (screens have none outside strings). A `#` at the start of a code line is a preprocessor directive, but colors are always in quotes, so it doesn't get in the way. The C99 standard only guarantees strings up to 4095 characters: a bigger screen (`-pedantic` warns) goes into a file, [lesson 5](05-files.md).

## Try it

- Add a "−1" button with a `dec` action.
- Make a mistake in the JSON (`txt` instead of `text`) and see what `uison_kit_fail` prints.

Next: [lesson 2: data and bindings](02-data.md).
