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
