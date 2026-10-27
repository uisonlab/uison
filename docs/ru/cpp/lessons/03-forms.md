# Урок 3. Форма с проверкой

[English](../../../cpp/lessons/03-forms.md) · **Русский**

**Что будет:** регистрация — имя, email, согласие. Ошибки появляются после первой попытки отправить и исчезают по мере исправления; фокус встаёт на первое поле с ошибкой.

![Урок 3](../../../cpp/img/03-forms.png)

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

## Как это работает

1. **Форма — структура.** `d.get<Form>("form")` читает объект `form` в структуру, `d.set("errors", Errors{…})` пишет структуру объектом. Поля и их имена находятся сами (C++20, без макросов): имя в данных — как у поля. Структура должна быть агрегатом в пространстве имён (не внутри функции).
2. **Привязки к полям.** `.value = "@form.name"` — путь внутрь объекта: структура в коде и данные экрана — одно и то же в двух видах.
3. **Ошибки.** У `TextField` есть `.error`: непустая строка — красная рамка и текст под полем. Ошибки — тоже данные (`errors`), их пересчитывает `validate`.
4. **Когда показывать.** `validate` ничего не делает, пока `tried` ложно; после первой попытки `derive` зовёт её на каждое изменение, и ошибки исчезают, как только поле исправлено.
5. **Фокус.** `cx.focus("name/1/0")` ставит каретку в поле: `TextField` — элемент из нескольких узлов, само поле ввода — его часть `name/1/0`. Enter в поле email (`.on_submit`) — то же, что кнопка.

## Попробуйте

- Проверку длины имени (не меньше двух букв).
- Пароль: `.password = true` и проверка совпадения двух полей.

Дальше — [урок 4: списки](04-lists.md).
