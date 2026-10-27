/* The low level in C: a frame of primitives by hand (no Ui or JSON screens), a counter button.
   uison does the window, drawing, diff and hit testing; here are only the state and the frame. */
#include <stdio.h>
#include <string.h>

#include <uison.h>

typedef struct {
    int count;
    bool hover, down;
    char frame[2048];
} Counter;

static const char* render(Counter* c, uison_host* h) {
    char label[32];
    snprintf(label, sizeof label, "Pressed: %d", c->count);
    double w = 0, asc = 0, desc = 0;
    uison_host_measure(h, label, 17, true, NULL, &w, &asc, &desc);
    const char* fill = c->down ? "#0062CC" : c->hover ? "#0071EB" : "#007AFF";
    /* A 220×56 button centered in the 360×200 window; the text is centered in the button by its metrics. */
    snprintf(c->frame, sizeof c->frame,
             "{ size: [360, 200], background: \"#F5F5F7\", items: ["
             "{ rect: [70, 72, 220, 56], radius: 12, fill: \"%s\" },"
             "{ text: \"%s\", at: [%.1f, %.1f], size: 17, bold: true, color: \"#FFFFFF\" },"
             "{ hit: [70, 72, 220, 56], id: \"btn\", cursor: \"pointer\" } ] }",
             fill, label, 180 - w / 2, 100 - (asc + desc) / 2);
    return c->frame;
}

static const char* frame(void* user, uison_host* h) { return render(user, h); }

static const char* on_event(void* user, const char* ev, const char* hit, uison_host* h) {
    Counter* c = user;
    bool over = hit && strcmp(hit, "btn") == 0;
    bool hover = c->hover, down = c->down;
    int count = c->count;
    if (strstr(ev, "\"type\":\"move\"")) c->hover = over;
    else if (strstr(ev, "\"type\":\"leave\"")) c->hover = false;
    else if (strstr(ev, "\"type\":\"down\"")) c->down = over;
    else if (strstr(ev, "\"type\":\"up\"")) {
        if (c->down && over) c->count++;
        c->down = false;
    }
    if (hover == c->hover && down == c->down && count == c->count) return NULL; /* same frame */
    return render(c, h);
}

int main(void) {
    static Counter counter;
    uison_app app = {sizeof(uison_app), &counter, frame, on_event, NULL, NULL};
    return uison_run_app(&app, "Counter (low level, C)");
}
