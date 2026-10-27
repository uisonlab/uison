// A todo list in C++: the screen as widgets (uison_widgets.hpp, like in Flutter), tasks as a struct.
#include <uison_widgets.hpp>

using namespace uison::widgets;

struct Todo {
    int id;
    std::string title;
    bool done = false;
};

static uison::Node screen() {
    // A task row; `@t` is an item of todos, the filter is declarative through switch.
    auto item = Row({
        .padding = {6, 4},
        .gap = 10,
        .cross = "center",
        .key = "{@t.id}",
        .visible = uison::Json::raw(R"json({ switch: "@filter", cases: { active: "!@t.done", done: "@t.done" }, default: true })json"),
        .children = {
            Checkbox({.checked = "@t.done"}),
            Text("@t.title", {
                .color = uison::Json::raw(R"json({ switch: "@t.done", cases: { true: "$theme.muted" }, default: "$theme.text" })json"),
                .max_lines = 1,
                .flex = 1,
            }),
            IconButton("trash", {.size = "sm", .key = "del-{@t.id}", .on_click = "remove"}),
        },
    });
    auto card = Box({
        .padding = 20,
        .fill = "$theme.card",
        .stroke = "$theme.divider",
        .width = 1,
        .radius = "$theme.radiusLg",
        .w = 460,
        .child = Column({.gap = 14, .cross = "stretch", .children = {
            Row({.gap = 8, .cross = "center", .children = {
                Text("Todos", {.size = 22, .bold = true}),
                Badge("{@stats.left}", {.color = "$theme.primary", .visible = "@stats.left"}),
                Spacer({}),
                Text("{@stats.done} of {@stats.total} done", {.size = 12, .color = "$theme.muted"}),
            }}),
            Row({.gap = 8, .cross = "center", .children = {
                TextField({.value = "@draft", .placeholder = "What needs to be done?", .flex = 1, .key = "draft", .on_submit = "add"}),
                Button("Add", {.icon = "plus", .disabled = "!@draft", .on_click = "add"}),
            }}),
            Tabs({.value = "@filter", .tabs = uison::choices({{"all", "All"}, {"active", "Active"}, {"done", "Completed"}})}),
            Column({.cross = "stretch", .children = {For({.each = "@todos", .as = "t", .item = item})}}),
            Text("Nothing here yet: add your first todo", {.color = "$theme.muted", .align = "center", .visible = "!@todos"}),
            Divider({}),
            Row({.cross = "center", .children = {
                Text("Remaining: {@stats.left}", {.size = 12, .color = "$theme.muted"}),
                Spacer({}),
                Button("Clear completed", {.variant = "text", .size = "sm", .visible = "@stats.done", .on_click = "clear"}),
            }}),
        }}),
    });
    return Scroll({.w = "fill", .h = "fill", .child = Center({.h = "hug", .margin = {24, 0}, .child = card})});
}

int main() {
    uison::Ui ui(screen(), {{"size", uison::arr{520, 620}}});
    auto d = ui.data();
    d.set("filter", "all");
    d.set("draft", "");
    d.set("next", 4);
    d.set("todos", std::vector<Todo>{{1, "Buy bread"}, {2, "Write Task2.md", true}, {3, "Build the element library"}});
    // Derived data: the counters are recomputed after any change.
    ui.derive([](uison::Data& d) {
        auto todos = d.get<std::vector<Todo>>("todos");
        int done = 0;
        for (auto& t : todos) done += t.done;
        d.set("stats.total", todos.size());
        d.set("stats.done", done);
        d.set("stats.left", todos.size() - done);
    });
    ui.on("add", [](uison::Cx& cx) {
        auto d = cx.data();
        auto title = d.get<std::string>("draft");
        if (title.find_first_not_of(' ') == std::string::npos) return;
        int id = d.get<int>("next");
        d.push("todos", Todo{id, title});
        d.set("next", id + 1);
        d.set("draft", "");
        cx.focus("draft/1/0");
    });
    ui.on("remove", [](uison::Cx& cx) {
        auto d = cx.data();
        auto todos = d.get<std::vector<Todo>>("todos");
        std::erase_if(todos, [&](const Todo& t) { return t.id == cx.key_num(); });
        d.set("todos", todos);
    });
    ui.on("clear", [](uison::Cx& cx) {
        auto todos = cx.data().get<std::vector<Todo>>("todos");
        std::erase_if(todos, [](const Todo& t) { return t.done; });
        cx.data().set("todos", todos);
    });
    return ui.run("Todos (C++)");
}
