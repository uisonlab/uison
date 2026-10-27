/* Headless C ABI test: every function of uison.h, errors, drop, the cost of a call across the boundary.
 * Built and run by tools/c-test.sh (including with AddressSanitizer).
 * Cyrillic sample data is intentional: it checks that UTF-8 strings cross the ABI intact. */
#include <stdio.h>
#include <string.h>
#include <time.h>

#include <uison.h>

static int failed = 0;
#define CHECK(c)                                                       \
    do {                                                               \
        if (!(c)) {                                                    \
            fprintf(stderr, "%s:%d: FAILED: %s\n", __FILE__, __LINE__, #c); \
            failed++;                                                  \
        }                                                              \
    } while (0)
#define STREQ(a, b) CHECK(strcmp((a), (b)) == 0)

static const char* SCREEN =
    "{ data: { n: 1, name: \"Анна\", form: { email: \"a@b.c\", agree: true }, todos: [] },"
    "  root: { Column: { children: ["
    "    { Text: { key: \"title\", text: \"Hello, {@name}\" } },"
    "    { TextField: { key: \"name\", value: \"@name\" } },"
    "    { Button: { key: \"inc\", text: \"+1\", on: { click: \"inc\" } } },"
    "  ] } } }";

static int drops = 0;
static void count_drop(void* user) {
    (void)user;
    drops++;
}

static void inc(uison_cx* cx, void* user) {
    int* calls = (int*)user;
    (*calls)++;
    STREQ(uison_cx_action(cx), "inc");
    STREQ(uison_cx_event(cx), "code");
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
    /* Strings live until the end of the handler: two at once in one expression. */
    char buf[64];
    snprintf(buf, sizeof buf, "%s-%s", uison_data_str(d, "name"), uison_cx_action(cx));
    STREQ(buf, "Анна-inc");
    /* Nodes of the screen tree. */
    const char* t = uison_cx_node_get(cx, "title", "text");
    CHECK(t && strcmp(t, "\"Hello, {@name}\"") == 0);
    CHECK(uison_cx_node_set(cx, "title", "size", "20"));
    CHECK(!uison_cx_node_set(cx, "nope", "size", "20"));
    CHECK(uison_last_error() != NULL);
    CHECK(uison_cx_node_get(cx, "title", "nope") == NULL);
    CHECK(uison_cx_set_token(cx, "primary", "\"#FF3B30\""));
    CHECK(!uison_cx_set_token(cx, "primary", "{ bad"));
    uison_cx_focus(cx, "name/1/0");
    uison_cx_blur(cx);
    uison_cx_set_theme(cx, "dark");
    uison_cx_timer(cx, 1000, "inc");
    uison_cx_copy(cx, "copied");
    STREQ(uison_cx_key(cx), "");
}

static void derive(uison_data* d, void* user) {
    (void)user;
    uison_data_set_f64(d, "twice", uison_data_f64(d, "n") * 2);
}

/* CPU time (C89): precise enough for hundreds of thousands of calls. */
static double now_us(void) {
    return (double)clock() * 1e6 / CLOCKS_PER_SEC;
}

int main(void) {
    CHECK(uison_abi_version() == UISON_ABI_VERSION);

    /* Parse errors: NULL and a message with line:col. */
    CHECK(uison_ui_new("{ root: { Box: { fill: } } }") == NULL);
    CHECK(uison_last_error() && strstr(uison_last_error(), "1:"));
    CHECK(uison_ui_load("/nonexistent.json") == NULL);
    CHECK(strstr(uison_last_error(), "not found") != NULL);

    uison_ui* ui = uison_ui_new(SCREEN);
    CHECK(ui != NULL);
    uison_data* d = uison_ui_data(ui);

    /* Data: types, reading, writing. */
    CHECK(uison_data_type(d, "n") == UISON_NUMBER);
    CHECK(uison_data_type(d, "name") == UISON_STRING);
    CHECK(uison_data_type(d, "form") == UISON_OBJECT);
    CHECK(uison_data_type(d, "todos") == UISON_ARRAY);
    CHECK(uison_data_type(d, "form.agree") == UISON_BOOL);
    CHECK(uison_data_type(d, "missing") == UISON_NONE);
    STREQ(uison_data_str(d, "form.email"), "a@b.c");
    STREQ(uison_data_str(d, "n"), "1");
    STREQ(uison_data_str(d, "missing"), "");
    CHECK(uison_data_bool(d, "form.agree"));
    CHECK(!uison_data_bool(d, "todos"));
    CHECK(uison_data_len(d, "form") == 2);
    STREQ(uison_data_key(d, "form", 1), "agree");
    CHECK(uison_data_key(d, "form", 5) == NULL);
    uison_data_set_str(d, "form.email", "x@y.z");
    uison_data_set_bool(d, "form.agree", false);
    uison_data_set_null(d, "form.extra");
    CHECK(uison_data_type(d, "form.extra") == UISON_NULL);
    CHECK(uison_data_set_json(d, "obj", "{ a: [1, 2, { b: \"в\" }] }"));
    STREQ(uison_data_str(d, "obj.a.2.b"), "в");
    CHECK(!uison_data_set_json(d, "obj", "{ a: "));
    CHECK(strstr(uison_last_error(), "JSON") != NULL);
    CHECK(uison_data_push_json(d, "todos", "{ id: 1, title: \"хлеб\" }"));
    CHECK(uison_data_push_json(d, "todos", "{ id: 2, title: \"молоко\" }"));
    CHECK(uison_data_len(d, "todos") == 2);
    STREQ(uison_data_json(d, "todos.1"), "{\"id\":2,\"title\":\"молоко\"}");
    CHECK(uison_data_json(d, "missing") == NULL);
    CHECK(uison_data_remove(d, "todos.0"));
    STREQ(uison_data_str(d, "todos.0.title"), "молоко");
    CHECK(uison_data_remove(d, "form.extra"));
    CHECK(!uison_data_remove(d, "form.extra"));

    /* Handlers, derive, drop. */
    int calls = 0;
    uison_ui_on(ui, "inc", inc, &calls, count_drop);
    uison_ui_derive(ui, derive, NULL, count_drop);
    CHECK(uison_data_f64(d, "twice") == 2); /* derive is called right away */
    uison_ui_run_action(ui, "inc");
    CHECK(calls == 1);
    CHECK(uison_data_f64(d, "n") == 2);
    uison_ui_run_action(ui, "missing"); /* no handler: a warning, not a crash */
    CHECK(uison_ui_apply(ui, "{\"action\":\"inc\",\"event\":\"code\"}"));
    CHECK(calls == 2 && uison_data_f64(d, "n") == 3 && uison_data_f64(d, "twice") == 6);
    CHECK(uison_ui_apply(ui, "{\"data\":{\"n\":10}}"));
    CHECK(uison_data_f64(d, "n") == 10 && uison_data_f64(d, "twice") == 20);
    CHECK(!uison_ui_apply(ui, "{\"hello\":1}"));

    /* Shortcuts, themes, components, images. */
    CHECK(uison_ui_shortcut(ui, "cmd+n", "inc"));
    CHECK(!uison_ui_shortcut(ui, "cmd+", "inc"));
    uison_ui_set_theme(ui, "dark");
    CHECK(uison_ui_add_components(ui, "{ Hello: { params: { who: \"world\" }, body: { Text: { text: \"$who\" } } } }"));
    CHECK(uison_ui_component(ui, "Bye", "{ body: { Text: { text: \"bye\" } } }"));
    CHECK(!uison_ui_add_components(ui, "{ Bad: 1 }"));
    static const uint8_t png[] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1a, '\n', 0, 0, 0, 13, 'I', 'H', 'D', 'R', 0, 0, 0, 2, 0, 0, 0, 3};
    STREQ(uison_ui_image(ui, "dot", png, sizeof png), "mem:dot");

    /* The cost of a call across the boundary: get and set by path. */
    const int N = 200000;
    double t0 = now_us();
    double sum = 0;
    for (int i = 0; i < N; i++) sum += uison_data_f64(d, "form.agree") + uison_data_f64(d, "n");
    double t1 = now_us();
    for (int i = 0; i < N; i++) uison_data_set_f64(d, "bench.x", i);
    double t2 = now_us();
    printf("get f64: %.0f ns, set f64: %.0f ns (avg over %d)\n", (t1 - t0) * 1000 / (2.0 * N), (t2 - t1) * 1000 / N, N);
    (void)sum;

    uison_ui_free(ui);
    CHECK(drops == 2); /* the handler and derive are released together with ui */
    uison_ui_free(NULL);

    if (failed) {
        fprintf(stderr, "abi-test: %d check(s) failed\n", failed);
        return 1;
    }
    printf("abi-test: ok\n");
    return 0;
}
