# Lesson 3. A form with validation

**English** · [Русский](../../ru/cpp/lessons/03-forms.md)

**What you'll get:** a sign-up form — name, email, consent. Errors appear after the first attempt to submit and disappear as you fix the fields; focus moves to the first field with an error.

![Lesson 3](../img/03-forms.png)

<!-- code: examples-cpp/lessons/03-forms/main.cpp -->
```cpp
// Lesson 3: a form with validation. The form is a struct (data ↔ struct without macros, C++20);
// derive recomputes the errors after every change, submit puts focus on the first field with an error.
#include <string>

#include <uison_widgets.hpp>

using namespace uison::widgets;

struct Form {
    std::string name, email;
    bool agree = false;
};

struct Errors {
    std::string name, email;
};

static bool valid_email(const std::string& e) {
    auto at = e.find('@');
    return at != std::string::npos && at > 0 && e.find('.', at) != std::string::npos && e.back() != '.';
}

// Errors are shown after the first attempt to submit, then as the user types.
static void validate(uison::Data& d) {
    if (!d.get<bool>("tried")) return;
    auto f = d.get<Form>("form");
    d.set("errors", Errors{f.name.empty() ? "Enter your name" : "", valid_email(f.email) ? "" : "Check the address"});
}

int main() {
    auto submit = [](uison::Cx& cx) {
        auto d = cx.data();
        d.set("tried", true);
        validate(d);
        auto e = d.get<Errors>("errors");
        if (!e.name.empty()) cx.focus("name/1/0"); // the input inside the TextField "name"
        else if (!e.email.empty()) cx.focus("email/1/0");
        else {
            auto f = d.get<Form>("form");
            d.set("sent", "Done, " + f.name + "! We'll write to " + f.email);
        }
    };
    auto screen = Column({.padding = 24, .gap = 14, .cross = "stretch", .children = {
        Text("Sign up", {.size = 22, .bold = true}),
        TextField({.value = "@form.name", .label = "Name", .error = "@errors.name", .key = "name"}),
        TextField({.value = "@form.email", .label = "Email", .error = "@errors.email", .keyboard = "email", .key = "email",
                   .on_submit = submit}),
        Checkbox("I agree to the terms", {.checked = "@form.agree", .key = "agree"}),
        Button("Submit", {.key = "submit", .disabled = "!@form.agree", .on_click = submit}),
        Text("@sent", {.color = "$theme.success", .visible = "@sent"}),
    }});
    uison::Ui ui(screen, {.size = {420, 420}, .data = uison::obj{{"form", Form{}}, {"errors", Errors{}}, {"tried", false}, {"sent", ""}}});
    ui.derive(validate);
    return ui.run("Lesson 3 — form");
}
```

## How it works

1. **The form is a struct.** `d.get<Form>("form")` reads the `form` object into a struct, `d.set("errors", Errors{…})` writes a struct as an object. The fields and their names are found automatically (C++20, no macros): the name in the data is the field's name. The struct must be an aggregate at namespace scope (not inside a function).
2. **Bindings to fields.** `.value = "@form.name"` is a path into the object: the struct in code and the screen's data are the same thing in two forms.
3. **Errors.** `TextField` has `.error`: a non-empty string gives a red border and a text under the field. Errors are data too (`errors`), recomputed by `validate`.
4. **When to show them.** `validate` does nothing while `tried` is false; after the first attempt, `derive` calls it on every change, and an error disappears as soon as the field is fixed.
5. **Focus.** `cx.focus("name/1/0")` puts the caret into the field: `TextField` is an element made of several nodes, and the input itself is its part `name/1/0`. Enter in the email field (`.on_submit`) does the same as the button.

## Try it

- A check for the name's length (at least two letters).
- A password: `.password = true` and a check that two fields match.

Next: [lesson 4: lists](04-lists.md).
