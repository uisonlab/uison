/* uison_kit.h — JSON helpers for C programs. Header-only, C99, on top of uison.h;
 * costs nothing if you don't include it.
 *
 *   UISON_JSON({ … })                 screen JSON right in the code — no quote escaping
 *   UISON_ON(ui, add)                 handler: action name = function name
 *   uison_data_pushf(d, "todos", "{ id: %d, title: %s, done: %s }", id, t, UISON_B(done))
 *                                     data with substitution: %s is escaped, numbers are locale-independent
 *   UISON_INT(Todo, id) … UISON_END   field table: struct ↔ data in one call
 *   uison_kit_fail()                  print uison_last_error() and return 1
 *
 * Example:
 *
 *   uison_ui* ui = uison_ui_new(UISON_JSON({
 *       size: [320, 200],
 *       data: { n: 0 },
 *       root: { Column: { gap: 12, children: [
 *           { Text: { text: "Pressed {@n} times" } },
 *           { Button: { text: "+1", on: { click: "inc" } } },
 *       ] } }
 *   }));
 *   if (!ui) return uison_kit_fail();
 *   UISON_ON(ui, inc);
 */
#ifndef UISON_KIT_H
#define UISON_KIT_H

#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "uison.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ---------------------------------------------------------------- JSON in code */

/* The preprocessor turns the argument into a string and escapes quotes and "\" by itself. You write extended
   JSON (unquoted keys, trailing commas, // comments — the preprocessor strips them). Limitations:
   - parentheses outside strings must be balanced (screens have none outside strings);
   - the whole JSON becomes one line: a parse error always reports line 1, but shows the fragment around
     the spot (near "…{ Text: { txt <here> : 1 }…");
   - the preprocessor collapses whitespace between tokens to one space — inside JSON strings it's kept;
   - keep a large screen in a file (uison_ui_load): MSVC from Visual Studio 2022 stops at a string
     literal of ~16 KB (error C2026); Visual Studio 2026 compiled 92 KB. */
#define UISON_JSON(...) #__VA_ARGS__

/* ---------------------------------------------------------------- handlers */

/* uison_ui_on(ui, "inc", inc, NULL, NULL): the action name is the function name. */
#define UISON_ON(ui, f) uison_ui_on((ui), #f, (f), NULL, NULL)
#define UISON_ON_USER(ui, f, user, drop) uison_ui_on((ui), #f, (f), (user), (drop))

/* Print the last uison error to stderr and return 1: `if (!ui) return uison_kit_fail();` */
static inline int uison_kit_fail(void) {
    const char* e = uison_last_error();
    fprintf(stderr, "uison: %s\n", e ? e : "error");
    return 1;
}

/* ---------------------------------------------------------------- data with substitution */

/* The format is extended JSON with printf-style placeholders (the compiler checks the types):
     %s            a quoted, escaped string; NULL — null; UISON_B(x) — true / false
     %d %i %u %ld %lld %zu … integers
     %g %f %e      locale-independent numbers (%g without precision — shortest form); NaN and infinity — null
     %c            a character as a string; %% — a percent sign
   uison_fmt returns a malloc'ed string (free it with free), or NULL if out of memory. */
#if defined(__GNUC__) || defined(__clang__)
#define UISON_KIT_FMT(f, a) __attribute__((format(printf, f, a)))
#else
#define UISON_KIT_FMT(f, a)
#endif

/* bool for %s: written as true or false (not as a string). */
static inline const char* uison_kit_bool(int x) {
    static const char t[] = "true", f[] = "false";
    return x ? t : f;
}
#define UISON_B(x) uison_kit_bool((x) ? 1 : 0)

typedef struct uison_kit_buf {
    char* p;
    size_t len, cap;
    int oom;
} uison_kit_buf;

static inline void uison_kit_put(uison_kit_buf* b, const char* s, size_t n) {
    if (b->oom) return;
    if (b->len + n + 1 > b->cap) {
        size_t cap = b->cap ? b->cap * 2 : 128;
        char* p;
        while (cap < b->len + n + 1) cap *= 2;
        p = (char*)realloc(b->p, cap);
        if (!p) {
            b->oom = 1;
            return;
        }
        b->p = p;
        b->cap = cap;
    }
    memcpy(b->p + b->len, s, n);
    b->len += n;
    b->p[b->len] = 0;
}

/* Quoted JSON string: '"' and '\' are escaped, control characters become \n, \t, \u00XX. */
static inline void uison_kit_str(uison_kit_buf* b, const char* s, size_t n) {
    static const char hex[] = "0123456789abcdef";
    size_t i, from = 0;
    uison_kit_put(b, "\"", 1);
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        char esc[7] = {'\\', 0, 0, 0, 0, 0, 0};
        size_t len = 2;
        if (c == '"' || c == '\\') esc[1] = (char)c;
        else if (c == '\n') esc[1] = 'n';
        else if (c == '\r') esc[1] = 'r';
        else if (c == '\t') esc[1] = 't';
        else if (c < 0x20) {
            esc[1] = 'u', esc[2] = '0', esc[3] = '0', esc[4] = hex[c >> 4], esc[5] = hex[c & 15];
            len = 6;
        } else continue;
        uison_kit_put(b, s + from, i - from);
        uison_kit_put(b, esc, len);
        from = i + 1;
    }
    uison_kit_put(b, s + from, n - from);
    uison_kit_put(b, "\"", 1);
}

static inline void uison_kit_num(uison_kit_buf* b, double v, const char* spec, int prec) {
    char tmp[64];
    size_t n, i;
    if (v != v || v - v != 0) { /* NaN, infinity */
        uison_kit_put(b, "null", 4);
        return;
    }
    if (prec < 0) {
        n = uison_json_number(v, tmp);
    } else {
        /* Precision given: printf, then the locale's decimal comma → a dot. */
        char f[8] = {'%', '.', '*', spec[0], 0};
        int r = snprintf(tmp, sizeof tmp, f, prec, v);
        n = r < 0 ? 0 : (size_t)r < sizeof tmp ? (size_t)r : sizeof tmp - 1;
        for (i = 0; i < n; i++)
            if (tmp[i] == ',') tmp[i] = '.';
    }
    uison_kit_put(b, tmp, n);
}

static inline char* uison_vfmt(const char* fmt, va_list ap) {
    uison_kit_buf b = {NULL, 0, 0, 0};
    const char* p = fmt;
    uison_kit_put(&b, "", 0);
    while (*p) {
        const char* pct = strchr(p, '%');
        char len[3] = {0, 0, 0};
        int prec = -1;
        char tmp[32];
        if (!pct) {
            uison_kit_put(&b, p, strlen(p));
            break;
        }
        uison_kit_put(&b, p, (size_t)(pct - p));
        p = pct + 1;
        if (*p == '%') {
            uison_kit_put(&b, "%", 1);
            p++;
            continue;
        }
        /* Flags and width are meaningless for JSON — skip them; '*' consumes its argument. */
        while (*p && strchr("-+ #0", *p)) p++;
        if (*p == '*') (void)va_arg(ap, int), p++;
        while (*p >= '0' && *p <= '9') p++;
        if (*p == '.') {
            p++;
            prec = 0;
            if (*p == '*') prec = va_arg(ap, int), p++;
            while (*p >= '0' && *p <= '9') prec = prec * 10 + (*p++ - '0');
        }
        if (*p && strchr("hlLzjt", *p)) {
            len[0] = *p++;
            if ((len[0] == 'h' || len[0] == 'l') && *p == len[0]) len[1] = *p++;
        }
        switch (*p) {
        case 's': {
            const char* s = va_arg(ap, const char*);
            if (!s) uison_kit_put(&b, "null", 4);
            else if (s == uison_kit_bool(1)) uison_kit_put(&b, "true", 4);
            else if (s == uison_kit_bool(0)) uison_kit_put(&b, "false", 5);
            else uison_kit_str(&b, s, strlen(s));
            break;
        }
        case 'c': {
            char c = (char)va_arg(ap, int);
            uison_kit_str(&b, &c, 1);
            break;
        }
        case 'd':
        case 'i': {
            long long v;
            if (len[0] == 'l' && len[1] == 'l') v = va_arg(ap, long long);
            else if (len[0] == 'l') v = va_arg(ap, long);
            else if (len[0] == 'z') v = (long long)va_arg(ap, size_t);
            else if (len[0] == 'j') v = (long long)va_arg(ap, long long);
            else if (len[0] == 't') v = (long long)va_arg(ap, ptrdiff_t);
            else v = va_arg(ap, int);
            uison_kit_put(&b, tmp, (size_t)snprintf(tmp, sizeof tmp, "%lld", v));
            break;
        }
        case 'u':
        case 'x':
        case 'X':
        case 'o': {
            unsigned long long v;
            if (len[0] == 'l' && len[1] == 'l') v = va_arg(ap, unsigned long long);
            else if (len[0] == 'l') v = va_arg(ap, unsigned long);
            else if (len[0] == 'z') v = va_arg(ap, size_t);
            else if (len[0] == 'j') v = va_arg(ap, unsigned long long);
            else if (len[0] == 't') v = (unsigned long long)va_arg(ap, ptrdiff_t);
            else v = va_arg(ap, unsigned);
            uison_kit_put(&b, tmp, (size_t)snprintf(tmp, sizeof tmp, "%llu", v));
            break;
        }
        case 'f':
        case 'F':
        case 'e':
        case 'E':
        case 'g':
        case 'G': {
            double v = len[0] == 'L' ? (double)va_arg(ap, long double) : va_arg(ap, double);
            uison_kit_num(&b, v, p, prec);
            break;
        }
        default:
            /* Unknown — as is (and without consuming an argument). */
            uison_kit_put(&b, pct, (size_t)(p - pct) + (*p ? 1 : 0));
            if (!*p) p--;
        }
        p++;
    }
    if (b.oom) {
        free(b.p);
        return NULL;
    }
    return b.p;
}

static inline char* uison_fmt(const char* fmt, ...) UISON_KIT_FMT(1, 2);
static inline char* uison_fmt(const char* fmt, ...) {
    char* s;
    va_list ap;
    va_start(ap, fmt);
    s = uison_vfmt(fmt, ap);
    va_end(ap);
    return s;
}

/* Set the value at a path: uison_data_setf(d, "user", "{ name: %s, age: %d }", name, age). */
static inline bool uison_data_setf(uison_data* d, const char* path, const char* fmt, ...) UISON_KIT_FMT(3, 4);
static inline bool uison_data_setf(uison_data* d, const char* path, const char* fmt, ...) {
    bool ok;
    char* s;
    va_list ap;
    va_start(ap, fmt);
    s = uison_vfmt(fmt, ap);
    va_end(ap);
    ok = s && uison_data_set_json(d, path, s);
    free(s);
    return ok;
}

/* Append to an array: uison_data_pushf(d, "todos", "{ id: %d, title: %s }", id, title). */
static inline bool uison_data_pushf(uison_data* d, const char* path, const char* fmt, ...) UISON_KIT_FMT(3, 4);
static inline bool uison_data_pushf(uison_data* d, const char* path, const char* fmt, ...) {
    bool ok;
    char* s;
    va_list ap;
    va_start(ap, fmt);
    s = uison_vfmt(fmt, ap);
    va_end(ap);
    ok = s && uison_data_push_json(d, path, s);
    free(s);
    return ok;
}

/* ---------------------------------------------------------------- structs ↔ data */

/* Field table: JSON name = field name.
     typedef struct { int id; char title[64]; bool done; } Todo;
     static const uison_field TODO[] = { UISON_INT(Todo, id), UISON_CHARS(Todo, title), UISON_BOOL(Todo, done), UISON_END };
     uison_data_get_struct(d, "form", &form, FORM);         // object → struct
     uison_data_push_struct(d, "todos", &todo, TODO);       // struct → end of array
     UISON_SET_ARRAY(d, "rows", rows, n, ROW);              // array of structs → array of objects
   The field type is checked at compile time as far as the language allows: size (C99), type (C11, _Generic). */
#define UISON_KIT_CHECK(c) (0 * sizeof(char[(c) ? 1 : -1]))
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L && !defined(__cplusplus)
#define UISON_KIT_IS_INT(x)                                                                                            \
    _Generic((x), char: 1, signed char: 1, unsigned char: 1, short: 1, unsigned short: 1, int: 1, unsigned: 1,        \
             long: 1, unsigned long: 1, long long: 1, unsigned long long: 1, default: 0)
#define UISON_KIT_IS_REAL(x) _Generic((x), float: 1, double: 1, default: 0)
#define UISON_KIT_IS_BOOL(x) _Generic((x), _Bool: 1, default: 0)
#else
#define UISON_KIT_IS_INT(x) (sizeof(x) == 1 || sizeof(x) == 2 || sizeof(x) == 4 || sizeof(x) == 8)
#define UISON_KIT_IS_REAL(x) (sizeof(x) == 4 || sizeof(x) == 8)
#define UISON_KIT_IS_BOOL(x) (sizeof(x) == 1)
#endif
#define UISON_KIT_F(T, f) (((T*)0)->f)
#define UISON_FIELD(json, type, T, f, check, sub) {json, type, offsetof(T, f), sizeof(UISON_KIT_F(T, f)) + UISON_KIT_CHECK(check), sub}

#define UISON_INT(T, f) UISON_FIELD(#f, UISON_FIELD_INT, T, f, UISON_KIT_IS_INT(UISON_KIT_F(T, f)), NULL)
#define UISON_DOUBLE(T, f) UISON_FIELD(#f, UISON_FIELD_DOUBLE, T, f, UISON_KIT_IS_REAL(UISON_KIT_F(T, f)), NULL)
#define UISON_BOOL(T, f) UISON_FIELD(#f, UISON_FIELD_BOOL, T, f, UISON_KIT_IS_BOOL(UISON_KIT_F(T, f)), NULL)
/* char[N]: the string is cut at a UTF-8 character boundary, always NUL-terminated. */
#define UISON_CHARS(T, f) UISON_FIELD(#f, UISON_FIELD_CHARS, T, f, sizeof(UISON_KIT_F(T, f)[0]) == 1, NULL)
/* const char*: on read, a string that lives like uison_data_str strings. */
#define UISON_STR(T, f) UISON_FIELD(#f, UISON_FIELD_STR, T, f, sizeof(UISON_KIT_F(T, f)) == sizeof(char*), NULL)
/* Nested struct: table is its field table. */
#define UISON_STRUCT(T, f, table) UISON_FIELD(#f, UISON_FIELD_STRUCT, T, f, 1, table)
#define UISON_END {NULL, 0, 0, 0, NULL}

/* Array of structs: the element size is taken from the array. */
#define UISON_SET_ARRAY(d, path, items, n, table) uison_data_set_array((d), (path), (items), (n), sizeof *(items), (table))

#ifdef __cplusplus
}
#endif

#endif /* UISON_KIT_H */
