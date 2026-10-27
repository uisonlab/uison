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
