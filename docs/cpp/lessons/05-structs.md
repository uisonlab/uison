# Lesson 5. Data as structs

**English** · [Русский](../../ru/cpp/lessons/05-structs.md)

**What you'll get:** contacts from structs with a nested address and an optional phone number; a form adds a new one; the whole screen state is a struct too.

![Lesson 5](../img/05-structs.png)

<!-- code: examples-cpp/lessons/05-structs/main.cpp -->
```cpp
// Lesson 5 (C++20): data as structs. set/push/get understand any aggregate: fields, their names
// and nested structs are found automatically (no macros); plus vector, optional, map.
// The whole screen state is a struct too: {.data = State{…}}.
#include <optional>
#include <string>
#include <vector>

#include <uison_widgets.hpp>

using namespace uison::widgets;

struct Address {
    std::string city;
    std::string street;
};

struct Contact {
    int id = 0;
    std::string name;
    std::string email;
    std::optional<std::string> phone; // null in data when absent
    Address address;
    bool favorite = false;
};

struct Form {
    std::string name;
    std::string email;
};

// The whole screen state: fields are paths in data (form.name, contacts.0.email…).
struct State {
    std::vector<Contact> contacts = {};
    Form form = {};
    std::string summary = {};
};

int main() {
    auto add = [](uison::Cx& cx) {
        auto d = cx.data();
        auto f = d.get<Form>("form"); // data → struct
        if (f.name.empty()) return;
        int id = int(d.len("contacts")) + 1;
        d.push("contacts", Contact{id, f.name, f.email, std::nullopt, {"—", ""}}); // struct → data
        d.set("form", Form{});
    };
    auto screen = Column({.padding = 20, .gap = 12, .cross = "stretch", .children = {
        For({.each = "@contacts", .as = "c", .item = ListTile("{@c.name}{@c.star}", {
            .subtitle = "{@c.email} · {@c.address.city}", .leading = Avatar("@c.name", {.size = 32})})}),
        Row({.gap = 8, .children = {
            TextField({.value = "@form.name", .placeholder = "Name", .flex = 1, .key = "name"}),
            TextField({.value = "@form.email", .placeholder = "Email", .flex = 1, .key = "email"}),
            Button("Add", {.key = "add", .on_click = add}),
        }}),
        Text("@summary", {.size = 12, .color = "$theme.muted"}),
    }});
    State start{.contacts = {{1, "Alice", "alice@example.com", "+380 50 000 00 00", {"Kyiv", "Khreshchatyk, 1"}, true},
                             {2, "Brian", "brian@example.com", std::nullopt, {"Warsaw", "Nowy Swiat, 2"}}}};
    uison::Ui ui(screen, {.size = {460, 420}, .data = start});
    // Derived data: the star on favorites comes from structs read from data.
    ui.derive([](uison::Data& d) {
        auto contacts = d.get<std::vector<Contact>>("contacts");
        for (std::size_t i = 0; i < contacts.size(); i++)
            d.set("contacts." + std::to_string(i) + ".star", contacts[i].favorite ? " ★" : "");
        d.set("summary", "Contacts: " + std::to_string(contacts.size()) + ", first as JSON: " + d.json("contacts.0"));
    });
    return ui.run("Lesson 5 — structs");
}
```

## How it works

1. **Structs without macros.** `set`, `push`, `get<T>` understand any aggregate: the number of fields, their names and types are found automatically (C++20). Nested structs, `std::vector`, `std::map<std::string, T>`, `std::optional` (`null` in the data when empty) are supported.
2. **Screen state is a struct.** `{.data = State{…}}` takes the screen's initial data from a struct: the fields of `State` become paths (`contacts.0.email`, `form.name`).
3. **Where to declare it.** The struct goes at namespace scope (not inside a function): reflection needs external linkage.
4. **Your own conversion.** For a non-aggregate (a class with private fields), specialize `uison::convert<T>` with `write` (and `read` for `get<T>`).
5. **Derived data from structs.** `derive` reads a `std::vector<Contact>` and writes the stars and the summary into the data — the screen is bound to them.

## Try it

- A "Phone" field: the `std::optional<std::string>` is filled in only if the field is not empty.
- A "Favorite" mark on each contact: a `Checkbox` with `.checked = "@c.favorite"` right in the template.

Next: [lesson 6: custom components](06-components.md).
