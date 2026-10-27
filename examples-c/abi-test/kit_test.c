/* Headless uison_kit.h test: UISON_JSON, substitution with escaping, field tables.
 * Built twice: C99 and C11 (field type checks via _Generic).
 * Cyrillic strings are intentional: they test UTF-8 escaping and cutting at character boundaries. */
#include <math.h>
#include <stdio.h>
#include <string.h>

#include <uison_kit.h>

static int failed = 0;
#define CHECK(c)                                                            \
    do {                                                                    \
        if (!(c)) {                                                         \
            fprintf(stderr, "%s:%d: FAILED: %s\n", __FILE__, __LINE__, #c); \
            failed++;                                                       \
        }                                                                   \
    } while (0)
#define STREQ(a, b)                                                                              \
    do {                                                                                         \
        const char* a_ = (a);                                                                    \
        if (!a_ || strcmp(a_, (b)) != 0) {                                                       \
            fprintf(stderr, "%s:%d: FAILED: %s == %s (got %s)\n", __FILE__, __LINE__, #a, #b, a_); \
            failed++;                                                                            \
        }                                                                                        \
    } while (0)

typedef struct {
    double lat, lon;
} Point;
static const uison_field POINT[] = {UISON_DOUBLE(Point, lat), UISON_DOUBLE(Point, lon), UISON_END};

typedef struct {
    int id;
    char title[16];
    bool done;
    const char* note;
    float score;
    long long big;
    short small;
    Point at;
} Todo;
static const uison_field TODO[] = {
    UISON_INT(Todo, id),     UISON_CHARS(Todo, title), UISON_BOOL(Todo, done), UISON_STR(Todo, note),
    UISON_DOUBLE(Todo, score), UISON_INT(Todo, big),   UISON_INT(Todo, small), UISON_STRUCT(Todo, at, POINT),
    UISON_END,
};

typedef struct {
    char s[8];
} Short;
static const uison_field SHORT[] = {UISON_CHARS(Short, s), UISON_END};

static int pressed = 0;
static void inc(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
    pressed++;
}

int main(void) {
    /* JSON in code: unquoted keys, trailing commas, Cyrillic, "#", a comment. */
    uison_ui* ui = uison_ui_new(UISON_JSON({
        data: { n: 0, todos: [], color: "#FF0000", quote: "a \"b\" \\ c" },
        root: { Column: { gap: 12, children: [
            { Text: { text: "Нажато {@n} раз", size: 20 } },  // the preprocessor strips this comment
            { Button: { key: "inc", text: "+1", on: { click: "inc" } } },
        ] } }
    }));
    CHECK(ui != NULL);
    if (!ui) return uison_kit_fail();
    UISON_ON(ui, inc);
    uison_ui_run_action(ui, "inc");
    CHECK(pressed == 1);
    uison_data* d = uison_ui_data(ui);
    CHECK(uison_data_f64(d, "n") == 1);
    STREQ(uison_data_str(d, "color"), "#FF0000");
    STREQ(uison_data_str(d, "quote"), "a \"b\" \\ c");

    /* An error in single-line JSON comes with the fragment around the spot. */
    CHECK(uison_ui_new(UISON_JSON({ root: { Column: { gap: 12, children: [ { Text: { text: "x" ? } } ] } }, data: { long: "padding to make it long" } })) == NULL);
    CHECK(strstr(uison_last_error(), "<here>") != NULL);

    /* Substitution: strings are escaped, NULL is null, bool is true/false, numbers ignore the locale. */
    const char* tricky = "кавычка \" слэш \\ строка\nтаб\t";
    const char* none = uison_data_json(d, "missing"); /* NULL from the program (GCC warns about a literal NULL) */
    CHECK(uison_data_pushf(d, "todos", "{ id: %d, title: %s, done: %s, note: %s, x: %g, y: %.2f, z: %g, c: %c, pct: \"100%%\" }",
                           7, tricky, UISON_B(1), none, 0.1, 2.0 / 3, NAN, 'q'));
    STREQ(uison_data_str(d, "todos.0.title"), tricky);
    CHECK(uison_data_type(d, "todos.0.done") == UISON_BOOL && uison_data_bool(d, "todos.0.done"));
    CHECK(uison_data_type(d, "todos.0.note") == UISON_NULL);
    CHECK(uison_data_f64(d, "todos.0.x") == 0.1);
    STREQ(uison_data_str(d, "todos.0.y"), "0.67");
    CHECK(uison_data_type(d, "todos.0.z") == UISON_NULL);
    STREQ(uison_data_str(d, "todos.0.c"), "q");
    STREQ(uison_data_str(d, "todos.0.pct"), "100%");
    CHECK(uison_data_setf(d, "user", "{ name: %s, age: %zu, big: %lld, u: %u }", "Анна", (size_t)30, 1LL << 40, 7u));
    STREQ(uison_data_json(d, "user"), "{\"name\":\"Анна\",\"age\":30,\"big\":1099511627776,\"u\":7}");
    CHECK(!uison_data_setf(d, "bad", "{ x: %d", 1)); /* a parse error gives false */
    char* s = uison_fmt("[%s, %s, %d]", UISON_B(0), "ok", -3);
    STREQ(s, "[false, \"ok\", -3]");
    free(s);
    char num[32];
    uison_json_number(1e21, num);
    STREQ(num, "1000000000000000000000");
    uison_json_number(0.30000000000000004, num);
    STREQ(num, "0.30000000000000004");

    /* Struct → data → struct. */
    Todo t = {42, "ёжик", true, "заметка", 1.5f, 1LL << 40, -3, {55.75, 37.62}};
    CHECK(uison_data_set_struct(d, "todo", &t, TODO));
    STREQ(uison_data_str(d, "todo.title"), "ёжик");
    CHECK(uison_data_f64(d, "todo.at.lon") == 37.62);
    CHECK(uison_data_f64(d, "todo.big") == (double)(1LL << 40));
    Todo r;
    memset(&r, 0, sizeof r);
    r.id = -1;
    CHECK(uison_data_get_struct(d, "todo", &r, TODO));
    CHECK(r.id == 42 && r.done && r.score == 1.5f && r.big == (1LL << 40) && r.small == -3);
    STREQ(r.title, "ёжик");
    STREQ(r.note, "заметка");
    CHECK(r.at.lat == 55.75 && r.at.lon == 37.62);

    /* What's missing in data is left untouched. */
    CHECK(uison_data_set_json(d, "partial", UISON_JSON({ title: "новое", done: false, s: "ёёёё" })));
    Todo p = {5, "x", true, NULL, 0, 0, 0, {1, 2}};
    CHECK(uison_data_get_struct(d, "partial", &p, TODO));
    CHECK(p.id == 5 && !p.done && p.at.lat == 1);
    STREQ(p.title, "новое");
    CHECK(!uison_data_get_struct(d, "missing", &p, TODO));
    /* char[8]: at most 7 bytes, cut at a character boundary (ё is 2 bytes): "ёёё" and a null. */
    Short sh;
    CHECK(uison_data_get_struct(d, "partial", &sh, SHORT));
    STREQ(sh.s, "ёёё");

    /* Arrays of structs. */
    Todo list[3] = {{1, "a", false, NULL, 0, 0, 0, {0, 0}}, {2, "b", true, NULL, 0, 0, 0, {0, 0}}, {3, "c", false, NULL, 0, 0, 0, {0, 0}}};
    CHECK(UISON_SET_ARRAY(d, "list", list, 3, TODO));
    CHECK(uison_data_len(d, "list") == 3);
    CHECK(uison_data_bool(d, "list.1.done"));
    CHECK(uison_data_type(d, "list.0.note") == UISON_NULL);
    CHECK(uison_data_push_struct(d, "list", &t, TODO));
    CHECK(uison_data_f64(d, "list.3.id") == 42);

    uison_ui_free(ui);
    if (failed) {
        fprintf(stderr, "kit-test: %d check(s) failed\n", failed);
        return 1;
    }
#ifdef __STDC_VERSION__
    printf("kit-test (C%ld): ok\n", (long)(__STDC_VERSION__ / 100 % 100));
#else /* MSVC without /std:c11 does not define it */
    printf("kit-test (C): ok\n");
#endif
    return 0;
}
