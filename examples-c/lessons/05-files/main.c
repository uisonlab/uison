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
