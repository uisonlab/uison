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
