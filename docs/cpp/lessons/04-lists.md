# Lesson 4. Lists

**English** · [Русский](../../ru/cpp/lessons/04-lists.md)

**What you'll get:** a list of 10,000 rows with avatars. Clicking a row shows the name at the top, the trash button deletes the row.

![Lesson 4](../img/04-lists.png)

<!-- code: examples-cpp/lessons/04-lists/main.cpp -->
```cpp
// Lesson 4: lists. List draws only the visible rows, so 10,000 elements don't slow it down.
// The rows are a vector of structs; which row was pressed comes from the node's key (cx.key_num() is the number at its end).
#include <string>
#include <vector>

#include <uison_widgets.hpp>

using namespace uison::widgets;

struct Person {
    int id;
    std::string name;
};

// Index of the row with this id (after deletions index and id differ); -1 if none.
static long find(const uison::Data& d, long id) {
    for (std::size_t i = 0; i < d.len("rows"); i++)
        if (d.get<long>("rows." + std::to_string(i) + ".id") == id) return long(i);
    return -1;
}

int main() {
    auto pick = [](uison::Cx& cx) {
        auto d = cx.data();
        long i = find(d, cx.key_num()); // key "row-17" → 17
        if (i >= 0) d.set("picked", "Selected: " + d.get<std::string>("rows." + std::to_string(i) + ".name"));
    };
    auto remove = [](uison::Cx& cx) {
        auto d = cx.data();
        long i = find(d, cx.key_num()); // key "del-17"
        if (i >= 0) d.remove("rows." + std::to_string(i));
    };
    // The row template: @r is the current element of rows; one lambda for all rows.
    auto item = Row({.padding = {0, 16}, .gap = 10, .cross = "center", .key = "row-{@r.id}", .on_click = pick, .children = {
        Avatar("@r.name", {.size = 28}),
        Text("@r.name", {.flex = 1}),
        Text("#{@r.id}", {.size = 12, .color = "$theme.muted"}),
        IconButton("trash", {.size = "sm", .key = "del-{@r.id}", .on_click = remove}),
    }});
    auto screen = Column({.cross = "stretch", .w = "fill", .h = "fill", .children = {
        Box({.padding = {12, 16}, .child = Text("@picked", {.bold = true})}),
        Divider({}),
        List({.items = "@rows", .as = "r", .item_height = 44, .flex = 1, .key = "list", .item = item}),
    }});
    const char* names[] = {"Alice Parker", "Brian Evans", "Clara Smith", "David Owen", "Emma Collins"};
    std::vector<Person> rows;
    for (int i = 0; i < 10000; i++) rows.push_back({i, names[i % 5]});
    uison::Ui ui(screen, {.size = {440, 520}, .data = uison::obj{{"picked", "Pick a row"}}});
    ui.data().set("rows", rows); // vector of structs → array of objects
    return ui.run("Lesson 4 — lists");
}
```

## How it works

1. **Virtualization.** `List` is a scroll view that expands only the visible rows: 10,000 rows are no slower than twenty. All rows have the same height (`.item_height = 44`). `For` would expand them all — it is meant for dozens of rows.
2. **A vector of structs in one call.** `ui.data().set("rows", rows)` writes a `std::vector<Person>` as an array of objects (10,000 rows take about 10 ms).
3. **Which row was clicked.** There is one lambda for all rows, and the rows differ by `key`: `"row-{@r.id}"` → `row-17`. `cx.key_num()` is the number at the end of the key.
4. **Index and id.** After a deletion, row indices shift, but ids stay the same. That's why `find` looks a row up by id.
5. **Nested events.** Clicking the trash button calls `remove`, not `pick`: an event bubbles up from the clicked node to the first one that has a handler, and stops there.

## Try it

- A search field: `derive` builds a `visible` array from `rows` by substring, and `List` shows `@visible`.
- Highlight the selected row: `.fill` on the `Row` with a selector on `@picked_id`.

Next: [lesson 5: data as structs](05-structs.md).
