# Lesson 8. Threads

**English** · [Русский](../../ru/c/lessons/08-threads.md)

**What you'll get:** a "download" in a worker thread: a progress bar, a percentage, a "Cancel" button. The UI doesn't freeze.

![Lesson 8](../img/08-threads.png)

<!-- code: examples-c/lessons/08-threads/main.c -->
```c
/* Lesson 8: work in another thread. The UI is touched only through uison_post: the function runs
   on the UI thread (like a handler), and the screen updates itself. Cancellation is a shared flag. */
#define _POSIX_C_SOURCE 200809L /* nanosleep */
#include <stdint.h>
#include <stdio.h>

#include <uison_kit.h>

/* ---- a thin wrapper over system threads: pthread (macOS, Linux, Android) and _beginthreadex (Windows) */
#ifdef _WIN32
#include <process.h>
#include <windows.h>
typedef volatile LONG flag_t;
static void flag_set(flag_t* f, int v) { InterlockedExchange(f, v); }
static int flag_get(flag_t* f) { return InterlockedCompareExchange(f, 0, 0); }
static void sleep_ms(unsigned ms) { Sleep(ms); }
static unsigned __stdcall trampoline(void* arg);
static void thread_start(void (*f)(void*), void* arg);
#else
#include <pthread.h>
#include <stdatomic.h>
#include <time.h>
typedef atomic_int flag_t;
static void flag_set(flag_t* f, int v) { atomic_store(f, v); }
static int flag_get(flag_t* f) { return atomic_load(f); }
static void sleep_ms(unsigned ms) {
    struct timespec t = {ms / 1000, (long)(ms % 1000) * 1000000L};
    nanosleep(&t, NULL);
}
#endif

typedef struct {
    void (*f)(void*);
    void* arg;
} Start;
static Start start_args;

#ifdef _WIN32
static unsigned __stdcall trampoline(void* arg) {
    Start* s = arg;
    s->f(s->arg);
    return 0;
}
static void thread_start(void (*f)(void*), void* arg) {
    start_args = (Start){f, arg};
    CloseHandle((HANDLE)_beginthreadex(NULL, 0, trampoline, &start_args, 0, NULL));
}
#else
static void* trampoline(void* arg) {
    Start* s = arg;
    s->f(s->arg);
    return NULL;
}
static void thread_start(void (*f)(void*), void* arg) {
    pthread_t t;
    start_args = (Start){f, arg};
    pthread_create(&t, NULL, trampoline, &start_args);
    pthread_detach(t);
}
#endif

/* ---- the program */

typedef struct {
    uison_handle* ui; /* a reference to the UI for other threads */
    flag_t cancel;
} Download;

/* Runs on the UI thread: user is the step number (1…40). */
static void step(uison_cx* cx, void* user) {
    int i = (int)(intptr_t)user;
    char status[64];
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "progress", i / 40.0);
    snprintf(status, sizeof status, "Downloaded %d%%", i * 25 / 10);
    uison_data_set_str(d, "status", status);
}

static void finish(uison_cx* cx, void* user) {
    uison_data* d = uison_cx_data(cx);
    uison_data_set_bool(d, "busy", false);
    uison_data_set_str(d, "status", user ? "Cancelled" : "Done");
}

/* The worker thread: "downloads" in chunks and reports to the UI. */
static void worker(void* arg) {
    Download* dl = arg;
    for (int i = 1; i <= 40 && !flag_get(&dl->cancel); i++) {
        sleep_ms(50);
        uison_post(dl->ui, step, (void*)(intptr_t)i, NULL);
    }
    uison_post(dl->ui, finish, flag_get(&dl->cancel) ? (void*)1 : NULL, NULL);
}

static void start(uison_cx* cx, void* user) {
    Download* dl = user;
    uison_data_set_bool(uison_cx_data(cx), "busy", true);
    flag_set(&dl->cancel, 0);
    thread_start(worker, dl);
}

static void cancel(uison_cx* cx, void* user) {
    Download* dl = user;
    (void)cx;
    flag_set(&dl->cancel, 1);
}

int main(void) {
    static Download dl;
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [420, 240],
        data: { progress: 0, busy: false, status: "Press “Download”" },
        root: { Column: { padding: 24, gap: 14, cross: "stretch", children: [
            { Text: { text: "Downloading", size: 20, bold: true } },
            { ProgressBar: { value: "@progress" } },
            { Text: { text: "@status", color: "$theme.muted" } },
            { Row: { gap: 8, children: [
                { Button: { key: "start", text: "Download", icon: "arrowDown", disabled: "@busy", on: { click: "start" } } },
                { Button: { key: "cancel", text: "Cancel", variant: "secondary", visible: "@busy", on: { click: "cancel" } } },
            ] } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    dl.ui = uison_ui_handle(ui); /* take it before uison_run */
    UISON_ON_USER(ui, start, &dl, NULL);
    UISON_ON_USER(ui, cancel, &dl, NULL);
    return uison_run(ui, "Lesson 8 — threads");
}
```

## How it works

1. **The UI lives in one thread.** The data, the tree and the handlers live in the window's thread. Long work goes into another thread, otherwise the window stops responding.
2. **`uison_post`.** `uison_ui_handle(ui)` is a reference to the UI for other threads (get it before `uison_run`). `uison_post(handle, f, user, drop)` queues a function: it runs in the window's thread like an ordinary handler — with `cx` and the data — and the screen updates. Here `user` is the step number packed into a pointer (`(void*)(intptr_t)i`).
3. **Threads in C.** `threads.h` (C11) isn't available everywhere (not on macOS, not in older MSVC), so the lesson keeps a thin twenty-line wrapper: `pthread` on macOS, Linux, Android and `_beginthreadex` on Windows. The cancel flag is an `atomic_int` (C11) or `Interlocked…` on Windows: a plain variable shared by two threads is a data race.
4. **Handler data.** `UISON_ON_USER(ui, start, &dl, NULL)` — the `start` function gets `&dl` (the handle and the flag) in `user`.
5. **The browser.** WebAssembly has no threads: the lesson isn't built there, and `uison_post` works from the same thread.

## Try it

- A real download: `libcurl` in the worker thread, progress by bytes read.
- A "Retry" button after cancellation.

Next: [lesson 9: your own frame](09-frame.md).
