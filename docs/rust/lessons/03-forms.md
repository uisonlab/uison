# Lesson 3. A form with validation

**English** · [Русский](../../ru/rust/lessons/03-forms.md)

**What you'll get:** a sign-up form — name, email, consent. Errors appear after the first attempt to submit and disappear as you fix them; focus moves to the first field with an error.

![Lesson 3](../img/03-forms.png)

<!-- code: crates/uison/examples/03-forms.rs -->
```rust
//! Lesson 3: a form with validation. The form is a struct (`#[derive(Data)]`); derive recomputes the errors
//! after every data change, the submit handler puts focus on the first field with an error.
use uison::prelude::*;

#[derive(Data, Default)]
struct Form {
    name: String,
    email: String,
    agree: bool,
}

#[derive(Data, Default)]
struct Errors {
    name: String,
    email: String,
}

fn main() {
    let screen = column().padding(24).gap(14).cross("stretch").children([
        text("Sign up").size(22).bold(),
        text_field().key("name").label("Name").bind("form.name").error("@errors.name"),
        text_field().key("email").label("Email").bind("form.email").error("@errors.email").keyboard("email").on_submit(submit),
        checkbox("I agree to the terms").key("agree").bind("form.agree"),
        button("Submit").key("submit").disabled("!@form.agree").on_click(submit),
        text("@sent").color("$theme.success").visible("@sent"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 420.0);
    ui.data.set("form", Form::default());
    ui.data.set("errors", Errors::default());
    ui.data.set("tried", false);
    ui.data.set("sent", "");
    ui.derive(validate);
    run(ui, "Lesson 3 — form");
}

fn valid_email(e: &str) -> bool {
    match e.find('@') {
        Some(at) => at > 0 && e[at..].contains('.') && !e.ends_with('.'),
        None => false,
    }
}

/// Errors are shown after the first attempt to submit, then as the user types.
fn validate(d: &mut Data) {
    if !d.bool("tried") {
        return;
    }
    let f: Form = d.read("form").unwrap_or_default();
    let name = if f.name.is_empty() { "Enter your name" } else { "" };
    let email = if valid_email(&f.email) { "" } else { "Check the address" };
    d.set("errors", Errors { name: name.into(), email: email.into() });
}

fn submit(cx: &mut Cx) {
    cx.data.set("tried", true);
    validate(cx.data);
    let e: Errors = cx.data.read("errors").unwrap_or_default();
    if !e.name.is_empty() {
        cx.focus("name/1/0"); // the input inside the TextField "name"
    } else if !e.email.is_empty() {
        cx.focus("email/1/0");
    } else {
        let f: Form = cx.data.read("form").unwrap_or_default();
        cx.data.set("sent", format!("Done, {}! We'll write to {}", f.name, f.email));
    }
}
```

## How it works

1. **The form is a struct.** `#[derive(Data)]` teaches the struct to convert to data and back: `ui.data.set("form", Form::default())` stores the object `{ name, email, agree }`, and `d.read::<Form>("form")` reads it into the struct. Fields missing from the data are taken from `Default`. The name in the data is the field's name; for a different one use `#[data(rename = "isDone")]`, to keep a field out of the data — `#[data(skip)]`.
2. **Bindings to struct fields.** `bind("form.name")` is a path into the object. The struct in the code and the screen's data are the same thing, just in different forms.
3. **Errors.** `text_field` has an `error` property: a non-empty string gives a red border and a message under the field. Errors are data too (`errors`), recomputed by `validate`.
4. **When to show them.** `validate` does nothing while `tried` is false: until the first attempt the form is "clean". After that, `derive` calls it on every change, and errors disappear as soon as the field is fixed.
5. **Focus.** `cx.focus("name/1/0")` puts the caret into the field. `TextField` is a library element made of several nodes, and the input itself is its part `name/1/0` (the element's key and the path inside). Pressing Enter in the email field (`on_submit`) does the same as the button.
6. **`derive` and the handler.** `validate` is a plain function over `&mut Data`: both `derive` and `submit` call it (the latter to decide where to put the focus).

## Try it

- Check the name length (at least two letters).
- A password: `text_field().password()` and a check that two fields match.

Next — [lesson 4: lists](04-lists.md).
