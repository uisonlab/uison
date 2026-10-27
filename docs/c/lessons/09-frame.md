# Lesson 9. Your own frame

**English** · [Русский](../../ru/c/lessons/09-frame.md)

**What you'll get:** a field where a click places a blue dot, a dot counter and a "Clear" button — with no screens or elements: the program builds the frame of primitives itself.

![Lesson 9](../img/09-frame.png)

<!-- code: examples-c/lessons/09-frame/main.c -->
```c
/* Lesson 9: your own frame, without Ui and JSON screens. The program builds the frame from primitives
   (rectangles, circles, text, hit areas) and handles events itself.
   A click on the field places a dot, the "Clear" button removes them all. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <uison_kit.h>

#define MAX_DOTS 64

typedef struct {
    int n;
    double x[MAX_DOTS], y[MAX_DOTS];
    char frame[8192];
    size_t len;
} Canvas;

/* Append a piece of the frame (a string from uison_fmt, freed here). */
static void put(Canvas* c, char* s) {
    size_t n = s ? strlen(s) : 0;
    if (c->len + n + 1 < sizeof c->frame) {
        memcpy(c->frame + c->len, s, n);
        c->len += n;
        c->frame[c->len] = 0;
    }
    free(s);
}

/* The frame is JSON with placeholders: UISON_JSON without escaping, %g for numbers, %s for strings. */
static const char* render(Canvas* c, uison_host* h) {
    char label[40];
    double w = 0;
    snprintf(label, sizeof label, "Dots: %d", c->n);
    uison_host_measure(h, label, 13, false, NULL, &w, NULL, NULL);
    c->len = 0;
    put(c, uison_fmt(UISON_JSON({ size: [400, 300], background: "#F5F5F7", items: [
        { rect: [10, 10, 380, 230], radius: 10, fill: "#FFFFFF", stroke: "#D2D2D7", width: 1 },
        { hit: [10, 10, 380, 230], id: "field", cursor: "pointer" },
        { text: %s, at: [%g, 258], size: 13, color: "#86868B" },
        { rect: [290, 250, 100, 32], radius: 8, fill: "#FF3B30" },
        { text: "Clear", at: [323, 257], size: 13, bold: true, color: "#FFFFFF" },
        { hit: [290, 250, 100, 32], id: "clear", cursor: "pointer" }), label, 200 - w / 2));
    for (int i = 0; i < c->n; i++) {
        put(c, uison_fmt(","));
        put(c, uison_fmt(UISON_JSON({ circle: [%g, %g, 8], fill: "#007AFF" }), c->x[i], c->y[i]));
    }
    put(c, uison_fmt("] }"));
    return c->frame;
}

static const char* frame(void* user, uison_host* h) { return render(user, h); }

/* The event is JSON: {"type":"down","x":120,"y":80,"button":"left"}. strstr/strtod are enough here;
   the JSON fragment to search for is built with the same UISON_JSON: UISON_JSON("x":) is the string "\"x\":". */
static const char* on_event(void* user, const char* ev, const char* hit, uison_host* h) {
    Canvas* c = user;
    if (!strstr(ev, UISON_JSON("type":"down")) || !strstr(ev, UISON_JSON("button":"left")) || !hit)
        return NULL; /* nothing else changes the frame */
    if (strcmp(hit, "clear") == 0) {
        c->n = 0;
    } else if (strcmp(hit, "field") == 0 && c->n < MAX_DOTS) {
        const char* x = strstr(ev, UISON_JSON("x":));
        const char* y = strstr(ev, UISON_JSON("y":));
        if (!x || !y) return NULL;
        c->x[c->n] = strtod(x + 4, NULL);
        c->y[c->n] = strtod(y + 4, NULL);
        c->n++;
    }
    return render(c, h);
}

int main(void) {
    static Canvas canvas;
    uison_app app = {sizeof(uison_app), &canvas, frame, on_event, NULL, NULL};
    return uison_run_app(&app, "Lesson 9 — your own frame");
}
```

## How it works

1. **The low level.** `uison_run_app` instead of `uison_run`: your own application is a `uison_app` struct with `frame` and `on_event` functions. uison provides only the window, drawing, redrawing what changed, and hit testing.
2. **A frame is JSON of primitives.** `{ size, background, items: [ … ] }`: `rect` (`[x, y, w, h]`, `fill`, `stroke`, `width`, `radius`), `circle` (`[x, y, r]`), `line`, `tri`, `text` (`at` is the top left of the line, `size`, `bold`, `color`), `image`, `clip` (nested `items`), `hit` — an invisible mouse area with an `id` and a cursor.
3. **Formatting without escaping.** `uison_fmt(UISON_JSON({ text: %s, at: [%g, 258] }), label, x)`: the template is written as JSON (the preprocessor escapes the quotes), `%s` inserts a quoted string, `%g` a locale-independent number. The finished frame must live until the next call — here it's in the `Canvas` buffer.
4. **Events.** `on_event` gets the event as a JSON string (`{"type":"down","x":120,"y":80,"button":"left"}`) and `hit` — the id of the area under the cursor. If the frame changed, return the new one; if not — `NULL`. The JSON fragment for `strstr` is built with the same `UISON_JSON("x":)`.
5. **Text.** The platform measures the line width: `uison_host_measure(h, text, size, bold, family, &width, …)`.

## Try it

- Connect the dots with lines: `{ line: [x1, y1, x2, y2], color: "#007AFF", width: 2 }`.
- Dragging: remember the dot on `down`, move it on `move`, release it on `up`.

Next: [lesson 10: everywhere](10-everywhere.md).
