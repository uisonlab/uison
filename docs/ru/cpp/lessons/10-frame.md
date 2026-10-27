# Урок 10. Свой кадр

[English](../../../cpp/lessons/10-frame.md) · **Русский**

**Что будет:** поле, на котором щелчок ставит синюю точку, счётчик точек и кнопка «Clear» — без экранов и элементов: кадр из примитивов программа строит сама.

![Урок 10](../../../cpp/img/10-frame.png)

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

## Как это работает

1. **Нижний уровень.** Класс от `uison::App` с `frame` и `on_event`, запуск — `uison::run(app, "…")`. От uison — только окно, отрисовка, перерисовка изменившегося и hit-тест.
2. **Кадр.** `uison::FrameWriter(width, height, background)` строит его без JSON руками: `rect`, `circle`, `tri`, `poly` (многоугольник), `line`, `text`, `image`, `clip`/`end_clip`, `hit` (невидимая область мыши с id и курсором); стиль — структурами `Paint` (`.fill`, `.stroke`, `.width`, `.radius`), `TextStyle` (`.size`, `.color`, `.bold`…), `LineStyle`. `str()` — готовый кадр.
3. **События.** `on_event` получает `uison::Event` (`type`: `down`, `up`, `move`, `scroll`, `key`, `resize`…; `x`, `y`, `button`, `code`, `text`) и `hit` — id области под курсором. Вернуть `true` — кадр изменился, uison снова вызовет `frame`. Перерисуется только то, что изменилось.
4. **Текст.** Ширину строки меряет платформа: `host.measure(text, size).width`.
5. **Таймер.** `wants_tick_us()` — через сколько микросекунд прислать событие `tick` (анимации: 16000), `-1` — не нужен.

## Попробуйте

- Соединить точки линиями: `f.line(x1, y1, x2, y2, {.color = "#007AFF", .width = 2})`.
- Анимация: точка растёт от 0 до 8 за полсекунды — `wants_tick_us()` и время из `std::chrono`.

Дальше — [урок 11: C++17 и JSON](11-cpp17.md).
