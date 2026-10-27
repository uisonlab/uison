# uison in assembly

**English** · [Русский](ru/asm.md)

uison has a plain C API ([`uison.h`](../crates/uison-c/include/uison.h)), so any language that can call C functions can use it, assembly included. [`examples-asm/`](../examples-asm) holds two lessons from the [C series](c/README.md), rewritten by hand:

| lesson | what it shows | x86-64 (NASM) | AArch64 (GNU as) |
|---|---|---|---|
| [1. A window and a button](c/lessons/01-hello.md) | the screen is a JSON string in the data section, the `inc` action is a handler function | [`x64.asm`](../examples-asm/lessons/01-hello/x64.asm) | [`arm64.S`](../examples-asm/lessons/01-hello/arm64.S) |
| [9. Your own frame](c/lessons/09-frame.md) | no screen language: the program writes a frame of primitives into a buffer and handles mouse events itself | [`x64.asm`](../examples-asm/lessons/09-frame/x64.asm) | [`arm64.S`](../examples-asm/lessons/09-frame/arm64.S) |

These are the same programs as in C, C++ and Rust, and their screenshots match pixel for pixel on Windows (x86-64) and macOS (Apple Silicon): `tools\lessons-compare.ps1`, `tools/lessons-compare.sh`.

![Lesson 1](c/img/01-hello.png) ![Lesson 9](c/img/09-frame.png)

## Which file for which system

- **`x64.asm`** — NASM, Intel syntax. One source covers both x86-64 calling conventions: Windows x64 (`nasm -f win64`) and System V for Linux (`-f elf64`) and macOS on Intel (`-f macho64 --gprefix _`). The differences are a few macros at the top of the file plus `%ifdef WIN64` where a call passes a `double`.
- **`arm64.S`** — GNU as syntax through the C compiler (`cc -c arm64.S`, so the C preprocessor runs first). One source covers macOS (Apple Silicon) and Linux (AArch64, e.g. a Raspberry Pi). The calling convention is the same on both, so only names and addresses differ: `SYM(main)` is `_main` on macOS, `addr x0, label` is `adrp` + `@PAGEOFF` or `:lo12:`, and `L(name)` makes a file-local label.

Windows on ARM is not covered: its assembler (`armasm64`) uses a different syntax.

The calling conventions in brief:

| | integer and pointer arguments | `double` arguments | preserved registers | the stack at `call` |
|---|---|---|---|---|
| Windows x64 | `rcx rdx r8 r9`, then the stack | the slot of the argument's position: the third argument goes in `xmm2` | `rbx rbp rdi rsi r12–r15` | 16-byte aligned, plus 32 bytes of shadow space for the callee |
| System V x86-64 | `rdi rsi rdx rcx r8 r9`, then the stack | `xmm0`–`xmm7` counted separately | `rbx rbp r12–r15` | 16-byte aligned |
| AArch64 (AAPCS64) | `x0`–`x7` | `d0`–`d7` counted separately | `x19`–`x28`, `d8`–`d15` | 16-byte aligned |

The return value is in `rax`/`xmm0` (x86-64) or `x0`/`d0` (AArch64). `bool` is one byte; in an argument, zero the whole register.

## How the lessons work

**Lesson 1.** `main` passes the JSON screen to `uison_ui_new`, binds the action `"inc"` to the function `inc` with `uison_ui_on(ui, "inc", inc, NULL, NULL)` and opens the window with `uison_run`. On a click, uison calls `inc(cx, user)`. The handler gets the data with `uison_cx_data`, reads `n` with `uison_data_f64` and writes `n + 1` with `uison_data_set_f64`. The screen redraws itself, because the text `Pressed {@n} times` is bound to `n`.

**Lesson 9.** No screen language here. `main` passes `uison_run_app` a `uison_app` structure: six 8-byte fields (`size`, `user`, `frame`, `on_event`, `wants_tick_us`, `drop`) laid out with `dq`/`.quad`. `frame` and `on_event` return a frame, which is a JSON string such as `{ size: [400, 300], items: [{ rect: […] }, { circle: [x, y, 8], fill: "#007AFF" }, { hit: […], id: "clear" }] }`. The program builds that string in an 8 KB buffer with two helpers: `put` appends a string and `num` appends a number through `uison_json_number`, which gives the shortest form and needs no `printf`. Events arrive as JSON as well (`{"type":"down","x":120,"y":80,"button":"left"}`), and libc's `strstr` and `strtod` are enough to read them. `uison_host_measure` gives the width of the "Dots: N" label, which is used to center it.

All variadic C functions (`printf`, `uison_fmt`, `uison_data_setf`) are avoided on purpose. Their calling rules differ between systems: on macOS on Apple Silicon, variadic arguments go on the stack.

## Building

You need an assembler, a C compiler (it links the program and adds the C runtime that calls `main`), CMake, and the library: built from source (Rust) or a prebuilt package `dist/uison-<platform>`. Setting these up is described in [C lesson 0](c/lessons/00-setup.md). For x86-64 you also need [NASM](https://www.nasm.us): `winget install NASM.NASM`, `sudo apt install nasm` or `brew install nasm`.

```sh
cmake -S examples-asm -B out/asm -DUISON_PACKAGE=/path/to/uison-<platform>   # without UISON_PACKAGE — from source
cmake --build out/asm --config Release
```

CMake chooses the file by processor (`x64.asm` or `arm64.S`) and builds `asm-01-hello` and `asm-09-frame`. On Windows (Visual Studio generator) the programs are in `out/asm/Release`.

Without CMake, from the package (`$D` is the package folder):

```sh
# Windows x64, in "x64 Native Tools Command Prompt": the DLL (uison.dll goes next to the program)
nasm -f win64 x64.asm -o hello.obj
link hello.obj %D%\lib\uison.dll.lib msvcrt.lib ucrt.lib vcruntime.lib

# Linux x86-64
nasm -f elf64 x64.asm -o hello.o
cc hello.o $D/lib/libuison.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -o hello

# Linux AArch64
cc arm64.S $D/lib/libuison.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -o hello

# macOS (Apple Silicon)
cc arm64.S $D/lib/libuison.a -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip -o hello
```

`link` needs the C runtime libraries spelled out: `cl` writes "link the C runtime" into every object it compiles, while NASM doesn't. The static `uison.lib` also needs the system libraries listed in `UISON_SYSTEM_LIBS` in [`uison-libs.cmake`](../crates/uison-c/cmake/uison-libs.cmake). CMake adds them itself.

Testing works the same as for the other lessons: `UISON_SCRIPT=scenarios/lessons/hello.json` taps the button, and `UISON_CAPTURE=folder` saves a screenshot. On Windows, `tools\lessons.ps1 asm` builds and runs both lessons, and `tools\c-dist.ps1 -Test` checks them against the package when NASM is installed. On a Mac, `tools/lessons.sh asm` builds both lessons from `arm64.S` and runs their scenarios (screenshots in `out/lessons/asm`), and `tools/lessons-compare.sh` checks them against Rust, C and C++.
