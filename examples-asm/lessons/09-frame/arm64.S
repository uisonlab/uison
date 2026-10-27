// Lesson 9 in assembly (AArch64, GNU as syntax): your own frame, without Ui and JSON screens — the same program as
// docs/c/lessons/09-frame. The program writes the frame of primitives (rectangles, circles, text, hit areas)
// into a buffer itself and handles events itself. A click on the field places a dot, "Clear" removes them all.
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

#define MAX_DOTS 64

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

// bss name, size: zero-filled memory that takes no space in the file.
.macro bss name, size
#ifdef __APPLE__
    .zerofill __DATA, __bss, \name, \size, 3
#else
    .bss
    .p2align 3
\name: .zero \size
#endif
.endm

    .text
    .p2align 2

// ---------------------------------------------------------------- building the frame
// The frame is a JSON string: { size: [w, h], background, items: [{ rect }, { circle }, { text }, { hit }…] }.
// put and num append at [pos]; their own convention (not C): put — the string in x0, num — the number in d0.
// 8 KB of buf is plenty: about 600 bytes plus 50 per dot.

// put: append the NUL-terminated string at x0. Uses x0–x3.
put:
    addr x1, pos
    ldr x2, [x1]
L(put_copy):
    ldrb w3, [x0], #1
    strb w3, [x2]
    cbz w3, L(put_end)
    add x2, x2, #1
    b L(put_copy)
L(put_end):
    str x2, [x1]                    // pos stays on the NUL: the next put overwrites it
    ret

// num: append the number in d0 as JSON (uison_json_number: the shortest form, 13, 163.5859375).
num:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    addr x1, pos
    ldr x0, [x1]
    bl SYM(uison_json_number)       // uison_json_number(d0, out = x0) → x0 = length
    addr x1, pos
    ldr x2, [x1]
    add x2, x2, x0
    str x2, [x1]
    ldp x29, x30, [sp], #16
    ret

// render(host) → x0: the whole frame.
render:
    stp x29, x30, [sp, #-32]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    mov x19, x0                     // x19 = host

    // label = "Dots: " + count: the same put and num, written into label
    addr x0, label
    addr x1, pos
    str x0, [x1]
    addr x0, dots
    bl put
    addr x0, count
    ldr x0, [x0]
    scvtf d0, x0
    bl num

    // uison_host_measure(host, label, 13, false, NULL, &width, NULL, NULL): the label width, to center it
    mov x0, x19
    addr x1, label
    fmov d0, #13.0                  // the first double — d0, while the integers go on in x2…
    mov w2, #0                      // bold = false
    mov x3, #0                      // family = NULL: the default font
    addr x4, width
    mov x5, #0                      // ascent and descent are not needed
    mov x6, #0
    bl SYM(uison_host_measure)

    addr x0, buf
    addr x1, pos
    str x0, [x1]
    addr x0, head
    bl put
    addr x0, label
    bl put
    addr x0, head2
    bl put
    addr x0, width
    ldr d1, [x0]
    fmov d2, #0.5
    fmul d1, d1, d2
    mov x0, #200
    scvtf d0, x0
    fsub d0, d0, d1                 // 200 - width / 2
    bl num
    addr x0, head3
    bl put

    mov x20, #0                     // x20 = i
L(render_dot):
    addr x0, count
    ldr x1, [x0]
    cmp x20, x1
    b.hs L(render_close)
    addr x0, dot1
    bl put
    addr x0, xs
    ldr d0, [x0, x20, lsl #3]
    bl num
    addr x0, comma
    bl put
    addr x0, ys
    ldr d0, [x0, x20, lsl #3]
    bl num
    addr x0, dot2
    bl put
    add x20, x20, #1
    b L(render_dot)
L(render_close):
    addr x0, tail
    bl put
    addr x0, buf
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #32
    ret

// ---------------------------------------------------------------- uison_app

// frame(user, host): the first frame.
frame:
    mov x0, x1
    b render                        // render(host)

// on_event(user, event, hit, host) → a new frame, or NULL if nothing changed.
// The event is JSON: {"type":"down","x":120,"y":80,"button":"left"}; strstr and strtod are enough here.
on_event:
    stp x29, x30, [sp, #-48]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    str x21, [sp, #32]
    mov x19, x1                     // x19 = event
    mov x20, x2                     // x20 = hit: the id of the hit area under the cursor, or NULL
    mov x21, x3                     // x21 = host
    mov x0, x19
    addr x1, is_down
    bl SYM(strstr)
    cbz x0, L(ev_nothing)
    mov x0, x19
    addr x1, is_left
    bl SYM(strstr)
    cbz x0, L(ev_nothing)
    cbz x20, L(ev_nothing)             // nothing else changes the frame

    mov x0, x20
    addr x1, id_clear
    bl SYM(strcmp)
    cbnz w0, L(ev_field)
    addr x0, count                  // "Clear"
    str xzr, [x0]
    b L(ev_render)
L(ev_field):
    mov x0, x20
    addr x1, id_field
    bl SYM(strcmp)
    cbnz w0, L(ev_render)
    addr x0, count
    ldr x0, [x0]
    cmp x0, #MAX_DOTS
    b.hs L(ev_render)
    mov x0, x19
    addr x1, key_x
    bl SYM(strstr)
    cbz x0, L(ev_nothing)
    add x0, x0, #4                  // the number after "x":
    mov x1, #0
    bl SYM(strtod)                  // d0 = x
    addr x0, count
    ldr x1, [x0]
    addr x0, xs
    str d0, [x0, x1, lsl #3]
    mov x0, x19
    addr x1, key_y
    bl SYM(strstr)
    cbz x0, L(ev_nothing)
    add x0, x0, #4
    mov x1, #0
    bl SYM(strtod)
    addr x0, count
    ldr x1, [x0]
    addr x2, ys
    str d0, [x2, x1, lsl #3]
    add x1, x1, #1
    str x1, [x0]
L(ev_render):
    mov x0, x21
    bl render
    b L(ev_done)
L(ev_nothing):
    mov x0, #0
L(ev_done):
    ldr x21, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

    .globl SYM(main)
SYM(main):
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    addr x0, app
    addr x1, title
    bl SYM(uison_run_app)           // uison_run_app(&app, title): the window; w0 = exit code
    ldp x29, x30, [sp], #16
    ret

// ---------------------------------------------------------------- data

    .data
    .p2align 3
// struct uison_app: size, user, frame, on_event, wants_tick_us (no ticks), drop.
app: .quad 48, 0, frame, on_event, 0, 0

bss count, 8                        // dots placed
bss xs, MAX_DOTS * 8
bss ys, MAX_DOTS * 8
bss width, 8                        // the label width
bss pos, 8                          // where put and num write
bss label, 40
bss buf, 8192

    RODATA
head:
    .ascii "{ size: [400, 300], background: \"#F5F5F7\", items: [\n"
    .ascii "  { rect: [10, 10, 380, 230], radius: 10, fill: \"#FFFFFF\", stroke: \"#D2D2D7\", width: 1 },\n"
    .ascii "  { hit: [10, 10, 380, 230], id: \"field\", cursor: \"pointer\" },\n"
    .asciz "  { text: \""
head2:
    .asciz "\", at: ["
head3:
    .ascii ", 258], size: 13, color: \"#86868B\" },\n"
    .ascii "  { rect: [290, 250, 100, 32], radius: 8, fill: \"#FF3B30\" },\n"
    .ascii "  { text: \"Clear\", at: [323, 257], size: 13, bold: true, color: \"#FFFFFF\" },\n"
    .asciz "  { hit: [290, 250, 100, 32], id: \"clear\", cursor: \"pointer\" }"
dot1:  .asciz ",\n  { circle: ["
comma: .asciz ", "
dot2:  .asciz ", 8], fill: \"#007AFF\" }"
tail:  .asciz "\n] }"
dots:  .asciz "Dots: "

is_down:  .asciz "\"type\":\"down\""
is_left:  .asciz "\"button\":\"left\""
key_x:    .asciz "\"x\":"
key_y:    .asciz "\"y\":"
id_field: .asciz "field"
id_clear: .asciz "clear"
title:    .asciz "Lesson 9 — your own frame"

#ifndef __APPLE__
    .section .note.GNU-stack, "", %progbits
#endif
