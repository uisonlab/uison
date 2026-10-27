/*
 * uison.h — C API of the uison framework: screens in JSON, logic in C.
 *
 *   uison_ui* ui = uison_ui_new("{ data: { n: 0 }, root: { Button: { text: \"+1 ({@n})\", on: { click: \"inc\" } } } }");
 *   uison_ui_on(ui, "inc", inc, NULL, NULL);
 *   return uison_run(ui, "Counter");
 *
 * Rules:
 *  - input strings are NUL-terminated UTF-8; NULL is an empty string;
 *  - output strings belong to the library: in a handler they live until it returns; for the
 *    data of a uison_ui before launch — until uison_ui_free / uison_run. Need it longer — copy it;
 *  - an error is NULL / false, the message is in uison_last_error() (per thread);
 *  - uison_cx pointers and their uison_data are valid only inside the handler.
 *
 * Compiles as C99 and as C++.
 */
#ifndef UISON_H
#define UISON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) && defined(UISON_DLL)
#define UISON_API __declspec(dllimport)
#else
#define UISON_API
#endif

#define UISON_ABI_VERSION 1

typedef struct uison_ui uison_ui;     /* screen, handlers, data */
typedef struct uison_cx uison_cx;     /* handler context */
typedef struct uison_data uison_data; /* data by path: "form.email", "todos.3.done" */
typedef struct uison_handle uison_handle; /* handle to the app for other threads */

typedef void (*uison_handler)(uison_cx* cx, void* user);
typedef void (*uison_derive)(uison_data* data, void* user);
typedef void (*uison_drop)(void* user); /* frees user when it's no longer needed; may be NULL */

/* ---------------------------------------------------------------- general */

UISON_API uint32_t uison_abi_version(void);       /* the library's UISON_ABI_VERSION */
UISON_API const char* uison_last_error(void);     /* this thread's last error, or NULL */

/* ---------------------------------------------------------------- Ui */

/* Screen from JSON: { data, theme, components, safeArea, root }. NULL on error (with line:col). */
UISON_API uison_ui* uison_ui_new(const char* json);
/* Screen from a file: an absolute path, or relative to the program's folder, then the current folder. */
UISON_API uison_ui* uison_ui_load(const char* path);
UISON_API void uison_ui_free(uison_ui* ui);
/* Components: { Name: { params: {...}, body: {...} } }. */
UISON_API bool uison_ui_add_components(uison_ui* ui, const char* json);
UISON_API bool uison_ui_load_components(uison_ui* ui, const char* path); /* the same from a file */
UISON_API bool uison_ui_component(uison_ui* ui, const char* name, const char* json); /* { params, body } */
/* Handler of an action from `on: { click: "name" }`. drop(user) — when the handler is no longer needed. */
UISON_API void uison_ui_on(uison_ui* ui, const char* action, uison_handler f, void* user, uison_drop drop);
/* Derived data: f is called right away and after every data change. */
UISON_API void uison_ui_derive(uison_ui* ui, uison_derive f, void* user, uison_drop drop);
/* Keyboard shortcut: "cmd+n" (⌘ on Apple, Ctrl elsewhere), "cmd+shift+z", "escape". */
UISON_API bool uison_ui_shortcut(uison_ui* ui, const char* combo, const char* action);
/* Theme before launch: "system" (default), "light", "dark" or a custom one. */
UISON_API void uison_ui_set_theme(uison_ui* ui, const char* name);
/* Call an action from code as if an element had triggered it (headless tests). */
UISON_API void uison_ui_run_action(uison_ui* ui, const char* action);
/* A JSON message, as from outside: {"action":"save","event":"click","key":"ok","pos":[x,y],"clicks":1} runs the
   action as if the element had fired it, {"data":{"path":value}} sets data. false — not a known message. */
UISON_API bool uison_ui_apply(uison_ui* ui, const char* msg);
/* Data before launch: initial values, checks in tests. */
UISON_API uison_data* uison_ui_data(uison_ui* ui);
/* Image from PNG/JPEG bytes (copied). Returns the source "mem:name" for Image / Avatar. */
UISON_API const char* uison_ui_image(uison_ui* ui, const char* name, const uint8_t* bytes, size_t len);
/* The same from a file (next to the program, then the current folder); NULL — no such file. */
UISON_API const char* uison_ui_image_file(uison_ui* ui, const char* name, const char* path);
/* Open the window. Takes ownership of ui (no need to free it). Returns the exit code. */
UISON_API int uison_run(uison_ui* ui, const char* title);
/* Program icon (PNG bytes, copied) for the running window: the Dock on macOS, the title bar and taskbar
   on Windows, the taskbar on X11. Call before uison_run / uison_run_app; only the first call counts.
   The icon of the file itself (Finder, Explorer, the Linux menu) comes from packaging: an .app, an .exe resource,
   a .desktop file. */
UISON_API void uison_set_icon(const uint8_t* png, size_t len);

/* ---------------------------------------------------------------- data */

enum {
    UISON_NONE = -1, /* no such path */
    UISON_NULL = 0,
    UISON_BOOL = 1,
    UISON_NUMBER = 2,
    UISON_STRING = 3,
    UISON_ARRAY = 4,
    UISON_OBJECT = 5
};

UISON_API int uison_data_type(uison_data* d, const char* path);
UISON_API const char* uison_data_str(uison_data* d, const char* path);  /* not a string — "" (numbers — their JSON form) */
UISON_API double uison_data_f64(uison_data* d, const char* path);       /* not a number — 0 */
UISON_API bool uison_data_bool(uison_data* d, const char* path);        /* truthiness as in @path bindings */
UISON_API size_t uison_data_len(uison_data* d, const char* path);       /* array length / number of object fields */
UISON_API const char* uison_data_key(uison_data* d, const char* path, size_t i); /* name of the i-th object field, or NULL */
UISON_API const char* uison_data_json(uison_data* d, const char* path); /* subtree as JSON, or NULL; "" — everything */
UISON_API void uison_data_set_str(uison_data* d, const char* path, const char* v);
UISON_API void uison_data_set_f64(uison_data* d, const char* path, double v);
UISON_API void uison_data_set_bool(uison_data* d, const char* path, bool v);
UISON_API void uison_data_set_null(uison_data* d, const char* path);
UISON_API bool uison_data_set_json(uison_data* d, const char* path, const char* json);
UISON_API bool uison_data_push_json(uison_data* d, const char* path, const char* json); /* to the end of an array */
UISON_API bool uison_data_remove(uison_data* d, const char* path); /* "todos.3" or "form.email" */

/* Structs ↔ data: a field table describes the struct layout (JSON name, type, offsetof, sizeof).
   Tables are easiest to write with the uison_kit.h macros: UISON_INT(Todo, id), UISON_CHARS(Todo, title)… UISON_END.
   INT — an integer of any width (1, 2, 4, 8 bytes), DOUBLE — float or double, BOOL — bool,
   CHARS — char[N] (cut at a UTF-8 boundary), STR — const char* (lives like uison_data_str strings),
   STRUCT — a nested struct (sub is its table). */
enum {
    UISON_FIELD_INT = 1,
    UISON_FIELD_DOUBLE = 2,
    UISON_FIELD_BOOL = 3,
    UISON_FIELD_CHARS = 4,
    UISON_FIELD_STR = 5,
    UISON_FIELD_STRUCT = 6
};
typedef struct uison_field {
    const char* name; /* JSON name; NULL — end of table */
    int type;         /* UISON_FIELD_* */
    size_t offset;
    size_t size;
    const struct uison_field* sub;
} uison_field;
/* Object at a path → struct: fields missing in the data are left untouched. false — not an object. */
UISON_API bool uison_data_get_struct(uison_data* d, const char* path, void* out, const uison_field* f);
UISON_API bool uison_data_set_struct(uison_data* d, const char* path, const void* v, const uison_field* f);
UISON_API bool uison_data_push_struct(uison_data* d, const char* path, const void* v, const uison_field* f);
/* n structs, stride bytes apart (sizeof(T) for an array T[]) — as an array of objects. */
UISON_API bool uison_data_set_array(uison_data* d, const char* path, const void* items, size_t n, size_t stride,
                                    const uison_field* f);
/* Number as locale-independent JSON: shortest form, NaN and infinity — null. out — at least 32 bytes. */
UISON_API size_t uison_json_number(double v, char out[32]);

/* ---------------------------------------------------------------- handler context */

UISON_API const char* uison_cx_action(uison_cx* cx);
UISON_API const char* uison_cx_event(uison_cx* cx); /* click, change, submit, menu, shortcut, timer, code */
UISON_API const char* uison_cx_key(uison_cx* cx);   /* key of the event's node: "del-3", "list/12" */
UISON_API uison_data* uison_cx_data(uison_cx* cx);
UISON_API void uison_cx_focus(uison_cx* cx, const char* key); /* text field: "name/1/0" inside TextField "name" */
UISON_API void uison_cx_blur(uison_cx* cx);
UISON_API void uison_cx_set_theme(uison_cx* cx, const char* name);
UISON_API bool uison_cx_set_token(uison_cx* cx, const char* name, const char* json); /* "primary", "\"#FF3B30\"" */
UISON_API void uison_cx_timer(uison_cx* cx, uint64_t ms, const char* action);
UISON_API void uison_cx_copy(uison_cx* cx, const char* text); /* to the system clipboard */
UISON_API void uison_cx_open_url(uison_cx* cx, const char* url);
UISON_API const char* uison_cx_node_get(uison_cx* cx, const char* key, const char* prop); /* JSON or NULL */
UISON_API bool uison_cx_node_set(uison_cx* cx, const char* key, const char* prop, const char* json);

/* ---------------------------------------------------------------- other threads */

/* Handle to the app: thread-safe; get it before uison_run, free with uison_handle_free. */
UISON_API uison_handle* uison_ui_handle(uison_ui* ui);
/* Run f(cx, user) on the UI thread (call from any thread); then a new frame.
   Before the window starts it runs right after launch. drop(user) — after it runs. */
UISON_API void uison_post(uison_handle* h, uison_handler f, void* user, uison_drop drop);
UISON_API void uison_handle_free(uison_handle* h);

/* ---------------------------------------------------------------- low level: your own frame */

/* Platform access during a frame/on_event call: text metrics, clipboard, links. */
typedef struct uison_host uison_host;

/* Your own app: a frame of primitives as a JSON string
   ({ size: [w, h], background: "#fff", items: [{ rect: [x, y, w, h], fill: "#…", radius }, { text: "…", at: [x, y] },
     { hit: [x, y, w, h], id: "btn", cursor: "pointer" }, …] }). The strings returned by frame and on_event
   belong to the app and must live until the next call. */
typedef struct uison_app {
    size_t size; /* sizeof(uison_app) — so the table can grow */
    void* user;
    const char* (*frame)(void* user, uison_host* host);
    /* event — JSON: {"type":"down","x":…,"y":…,"button":"left"}, move, up, leave, scroll (dx, dy), key
       (code, text, shift, ctrl, alt, cmd), resize (w, h), appearance (dark), tick, paste (text), wake.
       hit — id of the hit area under the cursor, or NULL. Return a new frame, or NULL if nothing changed. */
    const char* (*on_event)(void* user, const char* event, const char* hit, uison_host* host);
    int64_t (*wants_tick_us)(void* user); /* in how many µs to send a tick; -1 — not needed; may be NULL */
    uison_drop drop;
} uison_app;

UISON_API int uison_run_app(const uison_app* app, const char* title);
UISON_API void uison_host_measure(uison_host* h, const char* text, double size, bool bold, const char* family,
                              double* width, double* ascent, double* descent);
UISON_API void uison_host_copy(uison_host* h, const char* text);
UISON_API void uison_host_request_paste(uison_host* h); /* the answer is a paste event */
UISON_API bool uison_host_open_url(uison_host* h, const char* url);

#ifdef __cplusplus
}
#endif

#endif /* UISON_H */
