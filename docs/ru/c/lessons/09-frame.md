# Урок 9. Свой кадр

[English](../../../c/lessons/09-frame.md) · **Русский**

**Что будет:** поле, на котором щелчок ставит синюю точку, счётчик точек и кнопка «Clear» — без экранов и элементов: кадр из примитивов программа строит сама.

![Урок 9](../../../c/img/09-frame.png)

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

## Как это работает

1. **Нижний уровень.** `uison_run_app` вместо `uison_run`: своё приложение — структура `uison_app` с функциями `frame` и `on_event`. От uison — только окно, отрисовка, перерисовка изменившегося и hit-тест.
2. **Кадр — JSON примитивов.** `{ size, background, items: [ … ] }`: `rect` (`[x, y, w, h]`, `fill`, `stroke`, `width`, `radius`), `circle` (`[x, y, r]`), `line`, `tri`, `text` (`at` — левый верх строки, `size`, `bold`, `color`), `image`, `clip` (вложенные `items`), `hit` — невидимая область мыши с `id` и курсором.
3. **Подстановка без экранирования.** `uison_fmt(UISON_JSON({ text: %s, at: [%g, 258] }), label, x)`: шаблон пишется как JSON (кавычки экранирует препроцессор), `%s` подставляет строку в кавычках, `%g` — число без локали. Готовый кадр должен жить до следующего вызова — здесь он в буфере `Canvas`.
4. **События.** `on_event` получает событие JSON-строкой (`{"type":"down","x":120,"y":80,"button":"left"}`) и `hit` — id области под курсором. Кадр изменился — вернуть новый, нет — `NULL`. Кусок JSON для `strstr` — тем же `UISON_JSON("x":)`.
5. **Текст.** Ширину строки меряет платформа: `uison_host_measure(h, text, size, bold, family, &width, …)`.

## Попробуйте

- Соединить точки линиями: `{ line: [x1, y1, x2, y2], color: "#007AFF", width: 2 }`.
- Перетаскивание: запомнить точку на `down`, двигать на `move`, отпустить на `up`.

Дальше — [урок 10: везде](10-everywhere.md).
