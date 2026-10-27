; Lesson 9 in assembly (x86-64, NASM): your own frame, without Ui and JSON screens — the same program as
; docs/c/lessons/09-frame. The program writes the frame of primitives (rectangles, circles, text, hit areas)
; into a buffer itself and handles events itself. A click on the field places a dot, "Clear" removes them all.
; One source for Windows x64 (nasm -f win64) and System V: Linux (-f elf64), macOS on Intel (-f macho64 --gprefix _).

default rel

; ---------------------------------------------------------------- calling convention
; Integer and pointer arguments: Windows — rcx, rdx, r8, r9, then the stack (after 32 bytes of "shadow space"
; the caller always reserves); System V — rdi, rsi, rdx, rcx, r8, r9. A double argument on Windows takes
; the slot of its position (the third argument — xmm2), on System V the next free xmm register.
; Both keep rbx, rbp, r12–r15; Windows also rsi, rdi. The stack is aligned to 16 bytes at every call.
%ifidn __OUTPUT_FORMAT__, win64
  %define WIN64
  %define A1 rcx
  %define A2 rdx
  %define A3 r8
  %define A4 r9
  %define PLT
  %define RODATA .rdata
%else
  %define A1 rdi
  %define A2 rsi
  %define A3 rdx
  %define A4 rcx
  %define RODATA .rodata
  %ifidn __OUTPUT_FORMAT__, elf64
    %define PLT wrt ..plt     ; Linux programs are position-independent: calls to libc go through the PLT
  %else
    %define PLT
  %endif
%endif

MAX_DOTS equ 64

extern uison_run_app, uison_host_measure, uison_json_number
extern strstr, strcmp, strtod

section .text

; ---------------------------------------------------------------- building the frame
; The frame is a JSON string: { size: [w, h], background, items: [{ rect }, { circle }, { text }, { hit }…] }.
; put and num append to buf at [pos]; their own convention (not C): put — the string in rax, num — the number
; in xmm0. 8 KB is plenty: about 600 bytes plus 50 per dot.

; put: append the NUL-terminated string at rax. Uses rax, rcx, rdx.
put:
    mov rdx, [pos]
.copy:
    mov cl, [rax]
    test cl, cl
    jz .end
    mov [rdx], cl
    inc rax
    inc rdx
    jmp .copy
.end:
    mov byte [rdx], 0
    mov [pos], rdx
    ret

; num: append the number in xmm0 as JSON (uison_json_number: the shortest form, 13, 163.5859375).
num:
    sub rsp, 40                 ; shadow space; the return address plus 40 keeps the alignment
%ifdef WIN64
    mov rdx, [pos]              ; uison_json_number(v, out): Windows — out is in the second slot,
%else
    mov rdi, [pos]              ; System V — the first integer argument
%endif
    call uison_json_number PLT  ; rax = length
    add [pos], rax
    add rsp, 40
    ret

; render(host) → rax: the whole frame.
render:
    push rbx
    push r12
    sub rsp, 72                 ; shadow space and four stack arguments of uison_host_measure on Windows
    mov rbx, A1                 ; rbx = host

    ; label = "Dots: " + count
    mov dword [label], 'Dots'
    mov word [label + 4], ': '
    cvtsi2sd xmm0, qword [count]
%ifdef WIN64
    lea rdx, [label + 6]
%else
    lea rdi, [label + 6]
%endif
    call uison_json_number PLT  ; writes the number and the NUL

    ; uison_host_measure(host, label, 13, false, NULL, &width, NULL, NULL): the label width, to center it
%ifdef WIN64
    mov rcx, rbx
    lea rdx, [label]
    movsd xmm2, [size13]        ; the third slot
    xor r9d, r9d                ; bold = false
    mov qword [rsp + 32], 0     ; family = NULL: the default font
    lea rax, [width]
    mov [rsp + 40], rax
    mov qword [rsp + 48], 0     ; ascent and descent are not needed
    mov qword [rsp + 56], 0
%else
    mov rdi, rbx
    lea rsi, [label]
    movsd xmm0, [size13]        ; the first double
    xor edx, edx                ; bold
    xor ecx, ecx                ; family
    lea r8, [width]
    xor r9d, r9d                ; ascent
    mov qword [rsp], 0          ; descent: the seventh integer argument goes on the stack
%endif
    call uison_host_measure PLT

    lea rax, [buf]
    mov [pos], rax
    lea rax, [head]
    call put
    lea rax, [label]
    call put
    lea rax, [head2]
    call put
    movsd xmm1, [width]
    mulsd xmm1, [half]
    movsd xmm0, [center]
    subsd xmm0, xmm1            ; 200 - width / 2
    call num
    lea rax, [head3]
    call put

    xor r12d, r12d              ; r12 = i
.dot:
    cmp r12, [count]
    jae .close
    lea rax, [dot1]
    call put
    lea rax, [xs]
    movsd xmm0, [rax + r12 * 8]
    call num
    lea rax, [comma]
    call put
    lea rax, [ys]
    movsd xmm0, [rax + r12 * 8]
    call num
    lea rax, [dot2]
    call put
    inc r12
    jmp .dot
.close:
    lea rax, [tail]
    call put
    lea rax, [buf]
    add rsp, 72
    pop r12
    pop rbx
    ret

; ---------------------------------------------------------------- uison_app

; frame(user, host): the first frame.
frame:
    mov A1, A2
    jmp render                  ; render(host)

; on_event(user, event, hit, host) → a new frame, or NULL if nothing changed.
; The event is JSON: {"type":"down","x":120,"y":80,"button":"left"}; strstr and strtod are enough here.
on_event:
    push rbx
    push r12
    push r13
    sub rsp, 32
    mov rbx, A2                 ; rbx = event
    mov r12, A3                 ; r12 = hit: the id of the hit area under the cursor, or NULL
    mov r13, A4                 ; r13 = host
    mov A1, rbx
    lea A2, [is_down]
    call strstr PLT
    test rax, rax
    jz .nothing
    mov A1, rbx
    lea A2, [is_left]
    call strstr PLT
    test rax, rax
    jz .nothing
    test r12, r12
    jz .nothing                 ; nothing else changes the frame

    mov A1, r12
    lea A2, [id_clear]
    call strcmp PLT
    test eax, eax
    jnz .field
    mov qword [count], 0        ; "Clear"
    jmp .render
.field:
    mov A1, r12
    lea A2, [id_field]
    call strcmp PLT
    test eax, eax
    jnz .render
    cmp qword [count], MAX_DOTS
    jae .render
    mov A1, rbx
    lea A2, [key_x]
    call strstr PLT
    test rax, rax
    jz .nothing
    lea A1, [rax + 4]           ; the number after "x":
    xor A2, A2
    call strtod PLT             ; xmm0 = x
    mov rax, [count]
    lea rcx, [xs]
    movsd [rcx + rax * 8], xmm0
    mov A1, rbx
    lea A2, [key_y]
    call strstr PLT
    test rax, rax
    jz .nothing
    lea A1, [rax + 4]
    xor A2, A2
    call strtod PLT
    mov rax, [count]
    lea rcx, [ys]
    movsd [rcx + rax * 8], xmm0
    inc qword [count]
.render:
    mov A1, r13
    call render
    jmp .done
.nothing:
    xor eax, eax
.done:
    add rsp, 32
    pop r13
    pop r12
    pop rbx
    ret

global main
main:
    sub rsp, 40
    lea A1, [app]
    lea A2, [title]
    call uison_run_app PLT      ; uison_run_app(&app, title): the window; eax = exit code
    add rsp, 40
    ret

; ---------------------------------------------------------------- data

section .data
; struct uison_app: size, user, frame, on_event, wants_tick_us (no ticks), drop.
app: dq 48, 0, frame, on_event, 0, 0

section .bss
count: resq 1                   ; dots placed
xs:    resq MAX_DOTS
ys:    resq MAX_DOTS
width: resq 1                   ; the label width
pos:   resq 1                   ; where put and num write
label: resb 40
buf:   resb 8192

section RODATA
head:
    db '{ size: [400, 300], background: "#F5F5F7", items: [', 10
    db '  { rect: [10, 10, 380, 230], radius: 10, fill: "#FFFFFF", stroke: "#D2D2D7", width: 1 },', 10
    db '  { hit: [10, 10, 380, 230], id: "field", cursor: "pointer" },', 10
    db '  { text: "', 0
head2:
    db '", at: [', 0
head3:
    db ', 258], size: 13, color: "#86868B" },', 10
    db '  { rect: [290, 250, 100, 32], radius: 8, fill: "#FF3B30" },', 10
    db '  { text: "Clear", at: [323, 257], size: 13, bold: true, color: "#FFFFFF" },', 10
    db '  { hit: [290, 250, 100, 32], id: "clear", cursor: "pointer" }', 0
dot1:  db ',', 10, '  { circle: [', 0
comma: db ', ', 0
dot2:  db ', 8], fill: "#007AFF" }', 0
tail:  db 10, '] }', 0

is_down:  db '"type":"down"', 0
is_left:  db '"button":"left"', 0
key_x:    db '"x":', 0
key_y:    db '"y":', 0
id_field: db 'field', 0
id_clear: db 'clear', 0
title:    db 'Lesson 9 — your own frame', 0

align 8
size13: dq 13.0
half:   dq 0.5
center: dq 200.0

%ifidn __OUTPUT_FORMAT__, elf64
section .note.GNU-stack noalloc noexec nowrite progbits
%endif
