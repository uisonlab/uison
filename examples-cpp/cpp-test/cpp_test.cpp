// Headless uison.hpp test: data <-> C++ types, reflection (C++20), the builder, widgets (C++20), lambdas and drop.
// Cyrillic sample data is intentional: it checks that UTF-8 strings cross the C ABI intact and unescaped.
#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include <uison.hpp>
#if UISON_CPP >= 202002L
#include <uison_widgets.hpp>
#endif

static int failed = 0;
#define CHECK(c)                                                                 \
    do {                                                                         \
        if (!(c)) {                                                              \
            std::fprintf(stderr, "%s:%d: FAILED: %s\n", __FILE__, __LINE__, #c); \
            failed++;                                                            \
        }                                                                        \
    } while (0)

// scenarios/events.jsonl — the canonical event form written by Rust (uison_frame::event): every line must
// parse with the same type, numbers, key code and text. Skipped where the file can't be read (browser).
static void check_event_vectors() {
#ifdef UISON_EVENTS_FILE
    std::ifstream f(UISON_EVENTS_FILE, std::ios::binary);
    if (!f) {
        std::printf("cpp-test: %s not readable, event vectors skipped\n", UISON_EVENTS_FILE);
        return;
    }
    std::string line;
    int n = 0;
    auto field = [](const std::string& line, const char* key) -> const char* {
        auto i = line.find(std::string("\"") + key + "\":");
        return i == std::string::npos ? nullptr : line.c_str() + i + std::strlen(key) + 3;
    };
    while (std::getline(f, line)) {
        if (!line.empty() && line.back() == '\r') line.pop_back();
        const char* ty = field(line, "type");
        if (!ty) continue; // a message for uison-ui ({"action":…}, {"data":…})
        n++;
        auto e = uison::detail::parse_event(line.c_str());
        CHECK(e.type == std::string(ty + 1, std::strchr(ty + 1, '"')));
        for (auto [key, v] : {std::pair{"x", e.x}, {"y", e.y}, {"dx", e.dx}, {"dy", e.dy}, {"w", e.w}, {"h", e.h}})
            if (const char* p = field(line, key)) CHECK(v == std::strtod(p, nullptr));
        if (const char* p = field(line, "button")) CHECK(e.button == std::string(p + 1, std::strchr(p + 1, '"')));
        if (const char* p = field(line, "code")) CHECK(e.code == std::string(p + 1, std::strchr(p + 1, '"')));
        if (e.type == "paste") CHECK(e.text == "a\"b\\c\nd\te ж");
        if (line.find("\"cmd\":true") != std::string::npos) CHECK(e.cmd);
    }
    CHECK(n >= 33);
#endif
}

#if UISON_REFLECT
struct Todo {
    int id = 0;
    std::string title;
    bool done = false;
    std::optional<double> due;
};
struct Inner {
    int a;
    int b;
};
struct Outer {
    Inner in;
    std::vector<Todo> todos;
    std::map<std::string, int> counts;
};
#endif

struct Point {
    double x, y;
};
template <>
struct uison::convert<Point> {
    static void write(std::string& out, const Point& p) { out += uison::Json(uison::arr{p.x, p.y}).str(); }
    static Point read(const uison::Data& d, const std::string& path) { return {d.num(path + ".0"), d.num(path + ".1")}; }
};

#if UISON_REFLECT
// For reflection, at namespace scope: a struct inside a function has no linkage.
struct ScreenState {
    int n;
    std::string name;
};
#endif

int main() {
    // A parse error is an exception with line:col.
    try {
        uison::Ui bad("{ root: { Box: { fill: } } }");
        CHECK(false);
    } catch (const uison::Error& e) {
        CHECK(std::string(e.what()).find("1:") != std::string::npos);
    }

    uison::Ui ui(R"json({ data: { n: 1, name: "Анна", tags: ["a", "b"], form: { email: "x@y.z" } },
                        root: { Button: { text: "+1 ({@n})", on: { click: "inc" } } } })json");
    auto d = ui.data();
    CHECK(d.get<int>("n") == 1);
    CHECK(d.get<std::string>("name") == "Анна");
    CHECK(d.get<std::vector<std::string>>("tags") == (std::vector<std::string>{"a", "b"}));
    CHECK((d.get<std::map<std::string, std::string>>("form").at("email") == "x@y.z"));
    CHECK(!d.get<std::optional<int>>("missing"));
    CHECK(d.get<int>("missing", 7) == 7);
    d.set("pi", 3.25);
    d.set("s", std::string("строка \"в кавычках\"\n"));
    CHECK(d.get<std::string>("s") == "строка \"в кавычках\"\n");
    d.set("list", std::vector<int>{1, 2, 3});
    CHECK(d.len("list") == 3 && d.get<int>("list.2") == 3);
    d.push("list", 4);
    d.set("o", uison::obj{{"k", 1}, {"arr", uison::arr{true, nullptr, "x"}}});
    CHECK(d.json("o") == R"({"k":1,"arr":[true,null,"x"]})");
    d.set("p", Point{1.5, -2});
    CHECK(d.get<Point>("p").y == -2);
    d.set("small", 0.1);
    CHECK(d.json("small") == "0.1");
    d.update<int>("n", [](int n) { return n + 10; });
    CHECK(d.get<int>("n") == 11);

#if UISON_REFLECT
    d.push("todos", Todo{1, "хлеб", false, std::nullopt});
    d.push("todos", Todo{2, "молоко", true, 2.5});
    CHECK(d.json("todos.0") == R"({"id":1,"title":"хлеб","done":false,"due":null})");
    auto todos = d.get<std::vector<Todo>>("todos");
    CHECK(todos.size() == 2 && todos[1].title == "молоко" && todos[1].done && *todos[1].due == 2.5);
    Outer o{{1, 2}, todos, {{"x", 5}}};
    d.set("outer", o);
    auto o2 = d.get<Outer>("outer");
    CHECK(o2.in.b == 2 && o2.todos.size() == 2 && o2.counts.at("x") == 5);
    static_assert(uison::detail::count_fields<Todo>() == 4);
    static_assert(uison::detail::count_fields<Outer>() == 3);
    static_assert(uison::detail::field_name<Todo, 1>() == "title");
#endif

    // Handlers: stateful lambdas, an exception doesn't crash the program, drop on release.
    int calls = 0;
    auto alive = std::make_shared<int>(0);
    ui.on("inc", [&calls, alive](uison::Cx& cx) {
        calls++;
        CHECK(cx.action() == "inc");
        cx.data().update<int>("n", [](int n) { return n + 1; });
    });
    ui.on("throws", [] { throw std::runtime_error("oops"); });
    ui.derive([](uison::Data& dd) { dd.set("twice", dd.get<int>("n") * 2); });
    CHECK(alive.use_count() == 2);
    ui.run_action("inc");
    ui.run_action("throws");
    CHECK(calls == 1 && d.get<int>("n") == 12 && d.get<int>("twice") == 24);
    CHECK(ui.apply(R"({"action":"inc","event":"click","key":"b"})") && ui.apply(R"({"data":{"m":1}})"));
    CHECK(calls == 2 && d.get<int>("n") == 13 && d.get<int>("twice") == 26 && d.get<int>("m") == 1);

    // The screen builder with lambdas.
    int clicks = 0;
    uison::Ui built(uison::column().gap(8).children({
        uison::text("Hello").size(20).bold(),
        uison::button("Press").key("go").on_click([&] { clicks++; }),
        uison::for_each("items", "it", uison::text("@it.name")),
    }));
    CHECK(uison::column().gap(8).children({uison::text("x")}).json().str() == R"({"Column":{"gap":8,"children":[{"Text":{"text":"x"}}]}})");
    CHECK(uison::container().padding(4).child(uison::text("x")).json().str() == R"({"Box":{"padding":4,"child":{"Text":{"text":"x"}}}})");
    CHECK(uison::checkbox("Done").bind("t.done").json().str() == R"({"Checkbox":{"label":"Done","checked":"@t.done"}})");
    built.run_action("~h" + std::to_string(1)); // lambda names follow creation order (the first one is in this file)
    (void)clicks;

#if UISON_CPP >= 202002L
    // Widgets: property structs → the same JSON, lambdas → Ui handlers.
    {
        using namespace uison::widgets;
        CHECK(Column({.gap = 8, .children = {Text("x")}}).json().str() == R"({"Column":{"gap":8,"children":[{"Text":{"text":"x"}}]}})");
        CHECK(Box({.padding = {4, 8}, .child = Text("x", {.size = 12})}).json().str() ==
              R"({"Box":{"padding":[4,8],"child":{"Text":{"text":"x","size":12}}}})");
        CHECK(Checkbox({.checked = "@t.done", .label = "Done"}).json().str() == R"({"Checkbox":{"checked":"@t.done","label":"Done"}})");
        CHECK(Tabs({.value = "@f", .tabs = uison::choices({{"a", "A"}})}).json().str() ==
              R"({"Tabs":{"value":"@f","tabs":[{"value":"a","label":"A"}]}})");
        CHECK(Button("Save", {.on_click = "save"}).json().str() == R"({"Button":{"text":"Save","on":{"click":"save"}}})");
        // The same properties for two widgets: Prop is copied.
        TextProps muted{.size = 12, .color = "$theme.muted"};
        CHECK(Text("a", muted).json().str() == R"({"Text":{"text":"a","size":12,"color":"$theme.muted"}})");
        CHECK(Text("b", muted).json().str() == R"({"Text":{"text":"b","size":12,"color":"$theme.muted"}})");
        // A lambda on a nested widget is called by its own action name.
        int taps = 0;
        auto root = Center({.child = Button("+1", {.on_click = [&](uison::Cx& cx) { taps += cx.action().size() > 0; }})});
        std::string j = root.json().str();
        auto at = j.find("~h");
        std::string action = j.substr(at, j.find('"', at) - at);
        uison::Ui w(root);
        w.run_action(action);
        CHECK(taps == 1);
    }
#endif

    // Screen options as a struct (C++20: designated initializers; C++17: by field order).
    {
#if UISON_REFLECT
        uison::Ui s(uison::text("{@name}: {@n}"), {.size = {420, 380}, .data = ScreenState{3, "Анна"}, .theme = "dark"});
#else
        uison::Ui s(uison::text("{@name}: {@n}"), uison::Screen{{420, 380}, uison::obj{{"n", 3}, {"name", "Анна"}}, "dark"});
#endif
        CHECK(s.data().get<int>("n") == 3 && s.data().get<std::string>("name") == "Анна");
        uison::Ui t(uison::text("x"), uison::Screen{{}, nullptr, uison::obj{{"primary", "#FF3B30"}}, R"({ Hi: { body: { Text: { text: "hi" } } } })", false});
        CHECK(t.c() != nullptr);
    }

    // The low level: a frame without hand-written JSON and event parsing (locale-independent numbers, \uXXXX).
    {
        uison::FrameWriter f(400, 300, "#F5F5F7");
        f.rect(10, 10, 380, 230, {"#FFFFFF", "#D2D2D7", 1, 10}).text("Dots: 2", 12.5, 258, {13, "#86868B"});
        f.clip(0, 0, 100, 100).circle(50, 50, 8, {"#007AFF"}).end_clip().hit(290, 250, 100, 32, "clear");
        f.line(0, 0, 10, 10, {"#000", 2, false}).tri(0, 0, 1, 0, 0, 1, {"#f00"});
        CHECK(f.str() ==
              R"({"size":[400,300],"background":"#F5F5F7","items":[{"rect":[10,10,380,230],"fill":"#FFFFFF","stroke":"#D2D2D7","width":1,"radius":10},)"
              R"({"text":"Dots: 2","at":[12.5,258],"size":13,"color":"#86868B"},{"clip":[0,0,100,100],"items":[{"circle":[50,50,8],"fill":"#007AFF"}]},)"
              R"({"hit":[290,250,100,32],"id":"clear","cursor":"pointer"},{"line":[0,0,10,10],"color":"#000","width":2,"cap":"butt"},)"
              R"({"tri":[0,0,1,0,0,1],"fill":"#f00"}]})");
        uison::FrameWriter open(10, 10);
        open.clip(0, 0, 5, 5).rect(0, 0, 1, 1);
        CHECK(open.str() == R"({"size":[10,10],"items":[{"clip":[0,0,5,5],"items":[{"rect":[0,0,1,1]}]}]})");
        auto e = uison::detail::parse_event(R"({"type":"down","x":120.5,"y":-3e2,"button":"left"})");
        CHECK(e.type == "down" && e.x == 120.5 && e.y == -300 && e.button == "left");
        auto k = uison::detail::parse_event(R"({"type":"key","code":"char","text":"\u0436\"\n","shift":true,"ctrl":false,"alt":false,"cmd":true})");
        CHECK(k.code == "char" && k.text == "ж\"\n" && k.shift && !k.ctrl && k.cmd);
        auto r = uison::detail::parse_event(R"({"type":"resize","w":800,"h":600})");
        CHECK(r.w == 800 && r.h == 600);
        check_event_vectors();
    }

    {
        uison::Ui moved = std::move(ui);
        CHECK(alive.use_count() == 2);
    }
    CHECK(alive.use_count() == 1); // the lambda is released together with Ui

    if (failed) {
        std::fprintf(stderr, "cpp-test: %d check(s) failed\n", failed);
        return 1;
    }
    std::printf("cpp-test (__cplusplus %ld): ok\n", (long)UISON_CPP);
    return 0;
}
