# Lesson 3. A form with validation

**English** · [Русский](../../ru/c/lessons/03-forms.md)

**What you'll get:** a sign-up form — name, email, consent. Errors appear after the first attempt to submit and disappear as you fix them; focus moves to the first field with an error.

![Lesson 3](../img/03-forms.png)

<!-- code: examples-c/lessons/03-forms/main.c -->
```c
/* Lesson 3: a form with validation. The form is a struct (the FORM field table); derive recomputes the errors after
   every data change, the submit handler puts focus on the first field with an error. */
#include <stdio.h>
#include <string.h>

#include <uison_kit.h>

typedef struct {
    char name[64];
    char email[128];
    bool agree;
} Form;
/* The field table: the JSON name = the field name; the compiler checks type and size. */
static const uison_field FORM[] = {UISON_CHARS(Form, name), UISON_CHARS(Form, email), UISON_BOOL(Form, agree), UISON_END};

static bool valid_email(const char* e) {
    const char* at = strchr(e, '@');
    return at && at != e && strchr(at, '.') && e[strlen(e) - 1] != '.';
}

/* Errors are shown after the first attempt to submit, then as the user types. */
static void validate(uison_data* d, void* user) {
    Form f = {"", "", false};
    (void)user;
    if (!uison_data_bool(d, "tried")) return;
    uison_data_get_struct(d, "form", &f, FORM); /* the form object → struct */
    uison_data_set_str(d, "errors.name", f.name[0] ? "" : "Enter your name");
    uison_data_set_str(d, "errors.email", valid_email(f.email) ? "" : "Check the address");
}

static void submit(uison_cx* cx, void* user) {
    uison_data* d = uison_cx_data(cx);
    Form f;
    uison_data_set_bool(d, "tried", true);
    validate(d, user);
    if (uison_data_bool(d, "errors.name")) {
        uison_cx_focus(cx, "name/1/0"); /* the input inside the TextField "name" */
    } else if (uison_data_bool(d, "errors.email")) {
        uison_cx_focus(cx, "email/1/0");
    } else if (uison_data_get_struct(d, "form", &f, FORM)) {
        char msg[256]; /* plain text: snprintf; uison_fmt and setf are for JSON */
        snprintf(msg, sizeof msg, "Done, %s! We'll write to %s", f.name, f.email);
        uison_data_set_str(d, "sent", msg);
    }
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [420, 420],
        data: { form: { name: "", email: "", agree: false }, errors: { name: "", email: "" }, tried: false, sent: "" },
        root: { Column: { padding: 24, gap: 14, cross: "stretch", children: [
            { Text: { text: "Sign up", size: 22, bold: true } },
            { TextField: { key: "name", label: "Name", value: "@form.name", error: "@errors.name" } },
            { TextField: { key: "email", label: "Email", value: "@form.email", error: "@errors.email", keyboard: "email", on: { submit: "submit" } } },
            { Checkbox: { key: "agree", label: "I agree to the terms", checked: "@form.agree" } },
            { Button: { key: "submit", text: "Submit", disabled: "!@form.agree", on: { click: "submit" } } },
            { Text: { text: "@sent", color: "$theme.success", visible: "@sent" } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    uison_ui_derive(ui, validate, NULL, NULL);
    UISON_ON(ui, submit);
    return uison_run(ui, "Lesson 3 — form");
}
```

## How it works

1. **The form is a struct.** The `FORM` field table describes the layout of the `Form` struct: the JSON name (= the field name), type, offset, size. `uison_data_get_struct(d, "form", &f, FORM)` reads the `form` object into the struct in one call; `uison_data_set_struct` goes the other way.
2. **Field types** are `uison_kit.h` macros: `UISON_INT` (an integer of any width), `UISON_DOUBLE` (`float` or `double`), `UISON_BOOL`, `UISON_CHARS` (`char[N]`: the string is cut at a UTF-8 character boundary and is always null-terminated), `UISON_STR` (`const char*` — lives as long as `uison_data_str` strings), `UISON_STRUCT` (a nested struct with its own table), `UISON_END`. A wrong field type is a compile error (in C99 — by size, in C11 — by type via `_Generic`).
3. **What's missing from the data** leaves the struct field untouched: fill in the defaults, then read.
4. **Errors.** `TextField` has an `error` property: a non-empty string gives a red border and text under the field. Errors are data too (`errors`), recomputed by `validate`.
5. **When to show them.** `validate` does nothing while `tried` is false; after the first attempt `derive` calls it on every change, and an error disappears as soon as the field is fixed.
6. **Focus.** `uison_cx_focus(cx, "name/1/0")` puts the caret in the field. `TextField` is an element made of several nodes, and the input itself is its part `name/1/0`. Enter in the email field (`on: { submit: "submit" }`) does the same as the button.
7. **Text for people** — `snprintf`; `uison_fmt` and `setf` build JSON (their `%s` is a quoted string).

## Try it

- Check the name length (at least two letters).
- An "Age" field (`keyboard: "number"`) and `UISON_INT(Form, age)` in the table: the string from the field won't turn into a number by itself — check that too.

Next: [lesson 4: lists](04-lists.md).
