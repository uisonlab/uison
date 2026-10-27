# Lesson 5. Screens from files

**English** · [Русский](../../ru/c/lessons/05-files.md)

**What you'll get:** the screen and your own elements as JSON files next to the program: two counter cards and rows with "−" and "+" buttons. You can edit the files while the program is running.

![Lesson 5](../img/05-files.png)

<!-- code: examples-c/lessons/05-files/main.c -->
```c
/* Lesson 5: custom components. The screen and the components are files next to the program
   (CMake copies them into the build folder); one handler for several buttons, told apart by key. */
#include <stdio.h>
#include <string.h>

#include <uison.h>

/* The button key: "inc-Tasks" / "dec-Done" → the path in data. */
static const char* path_of(const char* key) {
    return strstr(key, "Tasks") ? "tasks" : "done";
}

static void change(uison_cx* cx, void* user) {
    int step = *(const int*)user;
    uison_data* d = uison_cx_data(cx);
    const char* p = path_of(uison_cx_key(cx));
    double v = uison_data_f64(d, p) + step;
    uison_data_set_f64(d, p, v < 0 ? 0 : v);
}

int main(void) {
    uison_ui* ui = uison_ui_load("screen.json");
    if (!ui || !uison_ui_load_components(ui, "components.json")) {
        fprintf(stderr, "%s\n", uison_last_error());
        return 1;
    }
    static const int plus = 1, minus = -1;
    uison_ui_on(ui, "inc", change, (void*)&plus, NULL);
    uison_ui_on(ui, "dec", change, (void*)&minus, NULL);
    return uison_run(ui, "Lesson 5 — components");
}
```

The screen is `screen.json`:

<!-- code: examples-c/lessons/05-files/screen.json -->
```js
// The lesson 5 screen: custom components Stat and Counter from components.json.
{
  size: [460, 300],
  data: { tasks: 3, done: 1 },
  root: { Column: { padding: 20, gap: 16, cross: "stretch", children: [
    { Row: { gap: 12, children: [
      { Stat: { title: "Tasks", value: "@tasks", icon: "check" } },
      { Stat: { title: "Done", value: "@done", icon: "star", color: "$theme.success" } },
    ] } },
    { Counter: { label: "Tasks" } },
    { Counter: { label: "Done" } },
  ] } }
}
```

Custom components are in `components.json`:

<!-- code: examples-c/lessons/05-files/components.json -->
```js
// Custom components: params are the parameters with default values, body is the tree,
// inside it parameters are $name. The screen uses them like built-in ones: { Stat: { title: … } }.
{
  Stat: {
    params: { title: "", value: 0, icon: "star", color: "$theme.primary" },
    body: {
      Box: {
        flex: 1, padding: 14, radius: "$theme.radius", fill: "$theme.card", stroke: "$theme.divider",
        child: { Column: { gap: 6, children: [
          { Row: { gap: 6, cross: "center", children: [
            { Icon: { name: "$icon", size: 16, color: "$color" } },
            { Text: { text: "$title", size: 12, color: "$theme.muted" } },
          ] } },
          { Text: { text: "$value", size: 26, bold: true } },
        ] } },
      },
    },
  },

  // The button keys are built from a parameter ("inc-{$label}"): each Counter has its own buttons,
  // while the handler is one; it tells which was pressed by the key.
  Counter: {
    params: { label: "" },
    body: {
      Row: { gap: 8, cross: "center", children: [
        { Text: { text: "$label", flex: 1 } },
        { IconButton: { icon: "minus", size: "sm", key: "dec-{$label}", on: { click: "dec" } } },
        { IconButton: { icon: "plus", size: "sm", key: "inc-{$label}", on: { click: "inc" } } },
      ] },
    },
  },
}
```

## How it works

1. **Files next to the program.** `uison_ui_load("screen.json")` looks for the file next to the program, then in the current folder; `uison_ui_load_components` does the same. CMake puts them next to the program (`uison_web_files` in [`examples-c/CMakeLists.txt`](../../../examples-c/CMakeLists.txt)); in the browser — into the page's file system.
2. **Live reload.** With `UISON_RELOAD=folder`, the screen and components are read from that folder (files with the same names) and rebuilt when you save, and the data is kept. It works on desktop (macOS, Windows, Linux):

   ```sh
   UISON_RELOAD=examples-c/lessons/05-files ./out/c/build/c-05-files
   ```

   Change a label or a color in `screen.json`, save — the window updates.
3. **Custom components.** `Stat` and `Counter` are templates: `params` (parameters with default values) and `body` (a tree; inside it parameters are `$name`). The screen uses them like built-in elements ([reference](../../screens/bindings.md#component-parameters-param)).
4. **One handler, different `user`.** `uison_ui_on(ui, "inc", change, &plus, NULL)` and `(ui, "dec", change, &minus, NULL)` — one function with a different step. Which row it is comes from the button key: `"inc-{$label}"` → `inc-Tasks`.
5. **Large screens** belong in files: you edit them without rebuilding, the editor highlights them as JSON, and their length isn't limited by the compiler's string literal limits.

## Try it

- Add a `Progress` component to `components.json` (a bar driven by `value` and `max`) and show it under the counters — without rebuilding the program.
- Keep everything in one file: put `components` right into `screen.json`.

Next: [lesson 6: themes and tokens](06-themes.md).
