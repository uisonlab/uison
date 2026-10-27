/* Lesson 4: lists. List draws only the visible rows, so 10,000 elements don't slow it down.
   The rows are an array of structs, put into data in one call (UISON_SET_ARRAY). Which row was pressed comes from the key. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <uison_kit.h>

typedef struct {
    int id;
    const char* name;
} Person;
static const uison_field PERSON[] = {UISON_INT(Person, id), UISON_STR(Person, name), UISON_END};

/* The number at the end of the key: "row-17" → 17. */
static long key_num(uison_cx* cx) {
    const char* dash = strrchr(uison_cx_key(cx), '-');
    return dash ? strtol(dash + 1, NULL, 10) : -1;
}

/* Index of the row with this id (after deletions index and id differ); -1 if none. */
static long find(uison_data* d, long id) {
    char path[64];
    for (size_t i = 0; i < uison_data_len(d, "rows"); i++) {
        snprintf(path, sizeof path, "rows.%zu.id", i);
        if ((long)uison_data_f64(d, path) == id) return (long)i;
    }
    return -1;
}

static void pick(uison_cx* cx, void* user) {
    uison_data* d = uison_cx_data(cx);
    long i = find(d, key_num(cx));
    char path[64], text[160];
    (void)user;
    if (i < 0) return;
    snprintf(path, sizeof path, "rows.%ld.name", i);
    snprintf(text, sizeof text, "Selected: %s", uison_data_str(d, path));
    uison_data_set_str(d, "picked", text);
}

static void remove_row(uison_cx* cx, void* user) {
    uison_data* d = uison_cx_data(cx);
    long i = find(d, key_num(cx));
    char path[64];
    (void)user;
    if (i < 0) return;
    snprintf(path, sizeof path, "rows.%ld", i);
    uison_data_remove(d, path);
}

int main(void) {
    static const char* names[] = {"Alice Parker", "Brian Evans", "Clara Smith", "David Owen", "Emma Collins"};
    static Person rows[10000];
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [440, 520],
        data: { picked: "Pick a row" },
        root: { Column: { w: "fill", h: "fill", cross: "stretch", children: [
            { Box: { padding: [12, 16], child: { Text: { text: "@picked", bold: true } } } },
            { Divider: {} },
            { List: {
                key: "list", flex: 1, items: "@rows", as: "r", itemHeight: 44,
                item: { Row: {
                    key: "row-{@r.id}", padding: [0, 16], gap: 10, cross: "center", on: { click: "pick" },
                    children: [
                        { Avatar: { name: "@r.name", size: 28 } },
                        { Text: { text: "@r.name", flex: 1 } },
                        { Text: { text: "#{@r.id}", size: 12, color: "$theme.muted" } },
                        { IconButton: { key: "del-{@r.id}", icon: "trash", size: "sm", on: { click: "remove_row" } } },
                    ] } } } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    for (int i = 0; i < 10000; i++) rows[i] = (Person){i, names[i % 5]};
    UISON_SET_ARRAY(uison_ui_data(ui), "rows", rows, 10000, PERSON); /* array of structs → array of objects */
    UISON_ON(ui, pick);
    UISON_ON(ui, remove_row);
    return uison_run(ui, "Lesson 4 — lists");
}
