// Lesson 1 in assembly (AArch64, GNU as syntax): a window with a counter, the same program as docs/c/lessons/01-hello.
// The screen is JSON in the data section, the logic is the handler inc; the rest are calls to uison.h.
// One source for macOS and Linux (cc -c arm64.S): the calling convention is the same — arguments in x0–x7,
// doubles in d0–d7 counted separately, x19–x28 and d8–d15 preserved; names and addresses differ — the macros below.

#ifdef __APPLE__
#define SYM(name) _##name           // C names on macOS start with _
#define L(name) L##name             // a file-local label: on macOS a conditional branch reaches only these
#define RODATA .section __TEXT,__const
#else
#define SYM(name) name
#define L(name) .L##name
#define RODATA .section .rodata
#endif

// addr reg, sym: the address of sym (a page with adrp, then the offset inside it).
.macro addr reg, sym
#ifdef __APPLE__
    adrp \reg, \sym\()@PAGE
    add \reg, \reg, \sym\()@PAGEOFF
#else
    adrp \reg, \sym
    add \reg, \reg, :lo12:\sym
#endif
.endm

    .text
    .p2align 2

// inc(cx, user): the "inc" action. Reads n, writes n + 1 — the screen updates itself.
inc:
    stp x29, x30, [sp, #-32]!
    mov x29, sp
    str x19, [sp, #16]              // x19 keeps d across calls
    bl SYM(uison_cx_data)           // x0 is still cx
    mov x19, x0                     // d = uison_cx_data(cx)
    addr x1, key_n
    bl SYM(uison_data_f64)          // d0 = n
    fmov d1, #1.0
    fadd d0, d0, d1
    mov x0, x19
    addr x1, key_n
    bl SYM(uison_data_set_f64)      // uison_data_set_f64(d, "n", n + 1): the double is in d0
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret

    .globl SYM(main)
SYM(main):
    stp x29, x30, [sp, #-32]!
    mov x29, sp
    str x19, [sp, #16]
    addr x0, screen
    bl SYM(uison_ui_new)            // x0 = ui, NULL on an error in the JSON
    cbz x0, L(fail)
    mov x19, x0
    addr x1, act_inc
    addr x2, inc
    mov x3, #0
    mov x4, #0
    bl SYM(uison_ui_on)             // uison_ui_on(ui, "inc", inc, NULL, NULL): action "inc" → function inc
    mov x0, x19
    addr x1, title
    bl SYM(uison_run)               // the window; w0 = exit code
    b L(done)
L(fail):
    bl SYM(uison_last_error)        // the message with line:col
    bl SYM(puts)
    mov w0, #1
L(done):
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret

    RODATA
// data is the data (one number n), root is the element tree; {@n} substitutes n, on.click names the action.
screen:
    .ascii "{ size: [320, 200], data: { n: 0 },\n"
    .ascii "  root: { Center: { child: { Column: { gap: 12, cross: \"center\", children: [\n"
    .ascii "    { Text: { text: \"Pressed {@n} times\", size: 20 } },\n"
    .ascii "    { Button: { key: \"inc\", text: \"+1\", on: { click: \"inc\" } } },\n"
    .asciz "  ] } } } } }"
key_n:   .asciz "n"
act_inc: .asciz "inc"
title:   .asciz "Lesson 1"

#ifndef __APPLE__
    .section .note.GNU-stack, "", %progbits
#endif
