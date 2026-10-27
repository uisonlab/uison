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
