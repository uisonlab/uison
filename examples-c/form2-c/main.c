/* form2 in C: the same screen as the Rust example examples/form2 (form.json), the logic is here. */
#include <stdio.h>
#include <string.h>

#include <uison.h>

#ifndef FORM_JSON
#define FORM_JSON "form.json"
#endif

static bool valid_email(const char* e) {
    const char* at = strchr(e, '@');
    if (!at || at == e) return false;
    const char* dot = strchr(at, '.');
    return dot && dot[1] != '\0' && e[strlen(e) - 1] != '.';
}

static bool blank(const char* s) {
    for (; *s; s++)
        if (*s != ' ' && *s != '\t') return false;
    return true;
}

/* Errors are shown after the first attempt to submit, then as the user types. */
static void validate(uison_data* d, void* user) {
    (void)user;
    if (!uison_data_bool(d, "tried")) return;
    uison_data_set_str(d, "errors.name", blank(uison_data_str(d, "form.name")) ? "Enter your name" : "");
    uison_data_set_str(d, "errors.email", valid_email(uison_data_str(d, "form.email")) ? "" : "Check the address");
}

static void submit(uison_cx* cx, void* user) {
    uison_data* d = uison_cx_data(cx);
    uison_data_set_bool(d, "tried", true);
    validate(d, user);
    if (uison_data_bool(d, "errors.name"))
        uison_cx_focus(cx, "name/1/0");
    else if (uison_data_bool(d, "errors.email"))
        uison_cx_focus(cx, "email/1/0");
    else
        uison_data_set_f64(d, "form.sent", uison_data_f64(d, "form.sent") + 1);
}

int main(void) {
    uison_ui* ui = uison_ui_load(FORM_JSON);
    if (!ui) {
        fprintf(stderr, "%s\n", uison_last_error());
        return 1;
    }
    uison_ui_derive(ui, validate, NULL, NULL);
    uison_ui_on(ui, "submit", submit, NULL, NULL);
    return uison_run(ui, "uison form2 (C)");
}
