; Lesson 1 in assembly (x86-64, NASM): a window with a counter, the same program as docs/c/lessons/01-hello.
; The screen is JSON in the data section, the logic is the handler inc; the rest are calls to uison.h.
; One source for both x86-64 calling conventions: Windows x64 (nasm -f win64) and System V —
; Linux (-f elf64), macOS on Intel (-f macho64 --gprefix _).

default rel

; ---------------------------------------------------------------- calling convention
; Integer and pointer arguments: Windows — rcx, rdx, r8, r9, then the stack (after 32 bytes of "shadow space"
; the caller always reserves); System V — rdi, rsi, rdx, rcx, r8, r9. Both keep rbx, rbp, r12–r15;
; Windows also rsi, rdi. The stack is aligned to 16 bytes at every call.
%ifidn __OUTPUT_FORMAT__, win64
  %define WIN64
  %define A1 rcx
  %define A2 rdx
  %define A3 r8
  %define A4 r9
  %define A5 qword [rsp + 32]
  %define PLT
  %define RODATA .rdata
%else
  %define A1 rdi
  %define A2 rsi
  %define A3 rdx
  %define A4 rcx
  %define A5 r8
  %define RODATA .rodata
  %ifidn __OUTPUT_FORMAT__, elf64
    %define PLT wrt ..plt     ; Linux programs are position-independent: calls to libc go through the PLT
  %else
    %define PLT
  %endif
%endif

extern uison_ui_new, uison_ui_on, uison_run, uison_last_error
extern uison_cx_data, uison_data_f64, uison_data_set_f64
extern puts

section .text

; inc(cx, user): the "inc" action. Reads n, writes n + 1 — the screen updates itself.
inc:
    push rbx                    ; rbx keeps d across calls; the push also aligns the stack to 16
    sub rsp, 32                 ; shadow space (Windows needs it; harmless on System V)
    call uison_cx_data PLT      ; A1 is still cx
    mov rbx, rax                ; d = uison_cx_data(cx)
    mov A1, rbx
    lea A2, [key_n]
    call uison_data_f64 PLT     ; xmm0 = n
    addsd xmm0, [one]
    mov A1, rbx
    lea A2, [key_n]
%ifdef WIN64
    movapd xmm2, xmm0           ; Windows: the third argument takes the third slot even when it is a double — xmm2
%endif                          ; System V: the first double argument is xmm0 — already there
    call uison_data_set_f64 PLT ; uison_data_set_f64(d, "n", n + 1)
    add rsp, 32
    pop rbx
    ret

global main
main:
    push rbx
    sub rsp, 48                 ; shadow space and the fifth argument on Windows; keeps the alignment
    lea A1, [screen]
    call uison_ui_new PLT       ; rax = ui, NULL on an error in the JSON
    test rax, rax
    jz .fail
    mov rbx, rax
    mov A1, rbx
    lea A2, [act_inc]
    lea A3, [inc]
    xor A4, A4
    mov A5, 0
    call uison_ui_on PLT        ; uison_ui_on(ui, "inc", inc, NULL, NULL): action "inc" → function inc
    mov A1, rbx
    lea A2, [title]
    call uison_run PLT          ; the window; eax = exit code
    jmp .done
.fail:
    call uison_last_error PLT   ; the message with line:col
    mov A1, rax
    call puts PLT
    mov eax, 1
.done:
    add rsp, 48
    pop rbx
    ret

section RODATA

; data is the data (one number n), root is the element tree; {@n} substitutes n, on.click names the action.
screen:
    db '{ size: [320, 200], data: { n: 0 },', 10
    db '  root: { Center: { child: { Column: { gap: 12, cross: "center", children: [', 10
    db '    { Text: { text: "Pressed {@n} times", size: 20 } },', 10
    db '    { Button: { key: "inc", text: "+1", on: { click: "inc" } } },', 10
    db '  ] } } } } }', 0
key_n:   db 'n', 0
act_inc: db 'inc', 0
title:   db 'Lesson 1', 0
align 8
one:     dq 1.0

%ifidn __OUTPUT_FORMAT__, elf64
section .note.GNU-stack noalloc noexec nowrite progbits
%endif
