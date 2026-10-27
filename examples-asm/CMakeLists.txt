# Assembly examples: lessons 1 (a counter) and 9 (your own frame) on the C API (uison.h).
#   cmake -S examples-asm -B out/asm/build && cmake --build out/asm/build
# x86-64 — NASM (Windows, Linux, macOS on Intel); AArch64 — GNU as through the C compiler (macOS, Linux).
# By default the library is built from source (needs Rust); -DUISON_PACKAGE=path/to/dist/uison-<platform>
# uses a prebuilt package (tools/c-dist.sh).
cmake_minimum_required(VERSION 3.19)
# C: the program is linked by the C compiler — it adds the C runtime that calls main.
project(uison_examples_asm LANGUAGES C)

if(CMAKE_SYSTEM_PROCESSOR MATCHES "^(x86_64|AMD64|amd64)$")
  enable_language(ASM_NASM) # the object format (win64, elf64, macho64) follows the platform
  if(APPLE)
    set(CMAKE_ASM_NASM_FLAGS "${CMAKE_ASM_NASM_FLAGS} --gprefix _") # C names on macOS start with _
  endif()
  set(UISON_ASM x64.asm)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$" AND NOT WIN32)
  enable_language(ASM) # .S: the C preprocessor, then the assembler
  set(UISON_ASM arm64.S)
else()
  message(FATAL_ERROR "examples-asm: x86-64 (Windows, Linux, macOS) or AArch64 (macOS, Linux), not ${CMAKE_SYSTEM_PROCESSOR}")
endif()

set(UISON_PACKAGE "" CACHE PATH "Prebuilt uison package (dist/uison-<platform>); empty: build from source")
if(UISON_PACKAGE)
  list(APPEND CMAKE_PREFIX_PATH "${UISON_PACKAGE}")
  find_package(uison REQUIRED)
else()
  add_subdirectory(../crates/uison-c uison)
endif()

# asm-NN-name: lessons/NN-name/x64.asm or arm64.S.
function(uison_asm_lesson name dir)
  add_executable(${name} ${dir}/${UISON_ASM})
  target_link_libraries(${name} PRIVATE uison::uison)
  set_target_properties(${name} PROPERTIES LINKER_LANGUAGE C)
  if(MSVC)
    # cl writes "link the C runtime" into every object it makes, NASM doesn't — name it here (/MD, CMake's default).
    target_link_libraries(${name} PRIVATE $<IF:$<CONFIG:Debug>,msvcrtd,msvcrt> $<IF:$<CONFIG:Debug>,ucrtd,ucrt>
      $<IF:$<CONFIG:Debug>,vcruntimed,vcruntime>)
  endif()
endfunction()

uison_asm_lesson(asm-01-hello lessons/01-hello)
uison_asm_lesson(asm-09-frame lessons/09-frame)
