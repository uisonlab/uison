# Урок 4. Списки

[English](../../../cpp/lessons/04-lists.md) · **Русский**

**Что будет:** список на 10 000 строк с аватарами. Нажатие на строку показывает имя сверху, корзина удаляет строку.

![Урок 4](../../../cpp/img/04-lists.png)

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

## Как это работает

1. **Виртуализация.** `List` — прокрутка, которая раскрывает только видимые строки: 10 000 строк не медленнее двадцати. Высота строк одна (`.item_height = 44`). `For` раскрыл бы все — он для десятков строк.
2. **Вектор структур — одним вызовом.** `ui.data().set("rows", rows)` записывает `std::vector<Person>` массивом объектов (10 000 строк — около 10 мс).
3. **Какая строка нажата.** Лямбда одна на все строки, а строки различаются `key`: `"row-{@r.id}"` → `row-17`. `cx.key_num()` — число в конце key.
4. **Номер и id.** После удаления номера строк сдвигаются, а id остаются. Поэтому `find` ищет строку по id.
5. **Вложенные события.** Нажатие на корзину вызывает `remove`, а не `pick`: событие поднимается от нажатого узла к первому, у кого есть обработчик, и дальше не идёт.

## Попробуйте

- Поле поиска: `derive` собирает массив `visible` из `rows` по подстроке, `List` показывает `@visible`.
- Выделить выбранную строку: `.fill` у `Row` селектором по `@picked_id`.

Дальше — [урок 5: данные как структуры](05-structs.md).
