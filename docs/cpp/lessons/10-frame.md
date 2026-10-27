# Lesson 10. Your own frame

**English** · [Русский](../../ru/cpp/lessons/10-frame.md)

**What you'll get:** a field where a click places a blue dot, a dot counter and a "Clear" button — no screens or elements: the program builds the frame from primitives itself.

![Lesson 10](../img/10-frame.png)

<!-- code: examples-cpp/lessons/10-frame/main.cpp -->
```cpp
// Lesson 10: your own frame, without Ui and screens. The program builds the frame from primitives (rectangles,
// circles, text, hit areas) and handles events itself: the uison::App class and the FrameWriter builder.
// A click on the field places a dot, the "Clear" button removes them all.
#include <string>
#include <vector>

#include <uison.hpp>

class Dots : public uison::App {
    struct Point {
        double x, y;
    };
    std::vector<Point> dots_;

public:
    std::string frame(uison::Host& host) override {
        uison::FrameWriter f(400, 300, "#F5F5F7");
        f.rect(10, 10, 380, 230, {.fill = "#FFFFFF", .stroke = "#D2D2D7", .width = 1, .radius = 10});
        f.hit(10, 10, 380, 230, "field");
        // A centered label: the platform measures the text width.
        auto label = "Dots: " + std::to_string(dots_.size());
        double w = host.measure(label, 13).width;
        f.text(label, 200 - w / 2, 258, {.size = 13, .color = "#86868B"});
        f.rect(290, 250, 100, 32, {.fill = "#FF3B30", .radius = 8});
        f.text("Clear", 323, 257, {.size = 13, .color = "#FFFFFF", .bold = true});
        f.hit(290, 250, 100, 32, "clear");
        for (auto& p : dots_) f.circle(p.x, p.y, 8, {.fill = "#007AFF"});
        return f.str();
    }

    // hit is the id of the hit area under the cursor; true means the frame changed (frame is called again).
    bool on_event(const uison::Event& e, std::string_view hit, uison::Host&) override {
        if (e.type != "down" || e.button != "left") return false;
        if (hit == "clear") dots_.clear();
        else if (hit == "field" && dots_.size() < 64) dots_.push_back({e.x, e.y});
        else return false;
        return true;
    }
};

int main() {
    Dots app;
    return uison::run(app, "Lesson 10 — your own frame");
}
```

## How it works

1. **The low level.** A class derived from `uison::App` with `frame` and `on_event`, launched with `uison::run(app, "…")`. uison provides only the window, drawing, redrawing what changed and hit testing.
2. **The frame.** `uison::FrameWriter(width, height, background)` builds it by hand without JSON: `rect`, `circle`, `tri`, `poly` (polygon), `line`, `text`, `image`, `clip`/`end_clip`, `hit` (an invisible mouse area with an id and a cursor); styles are the structs `Paint` (`.fill`, `.stroke`, `.width`, `.radius`), `TextStyle` (`.size`, `.color`, `.bold`…) and `LineStyle`. `str()` returns the finished frame.
3. **Events.** `on_event` receives a `uison::Event` (`type`: `down`, `up`, `move`, `scroll`, `key`, `resize`…; `x`, `y`, `button`, `code`, `text`) and `hit` — the id of the area under the cursor. Return `true` if the frame changed, and uison calls `frame` again. Only what changed is redrawn.
4. **Text.** The platform measures string width: `host.measure(text, size).width`.
5. **Timer.** `wants_tick_us()` — in how many microseconds to send a `tick` event (animations: 16000); `-1` — not needed.

## Try it

- Connect the dots with lines: `f.line(x1, y1, x2, y2, {.color = "#007AFF", .width = 2})`.
- Animation: a dot grows from 0 to 8 in half a second — `wants_tick_us()` and the time from `std::chrono`.

Next: [lesson 11: C++17 and JSON](11-cpp17.md).
