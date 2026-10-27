# C examples (C++ ones are in examples-cpp). By default the library is built from source (needs Rust);
# -DUISON_PACKAGE=path/to/dist/uison-<platform> uses a prebuilt package (tools/c-dist.sh).
#   cmake -S examples-c -B out/c/build && cmake --build out/c/build
cmake_minimum_required(VERSION 3.19)
project(uison_examples_c LANGUAGES C)

set(UISON_PACKAGE "" CACHE PATH "Prebuilt uison package (dist/uison-<platform>); empty: build from source")
if(UISON_PACKAGE)
  list(APPEND CMAKE_PREFIX_PATH "${UISON_PACKAGE}")
  # The iOS, NDK and Emscripten toolchains look for packages only in their sysroot, so the path is explicit.
  set(uison_DIR "${UISON_PACKAGE}/lib/cmake/uison")
  find_package(uison REQUIRED)
else()
  add_subdirectory(../crates/uison-c uison)
endif()

set(CMAKE_C_STANDARD 99)
if(MSVC)
  add_compile_options(/W4 /utf-8)
else()
  add_compile_options(-Wall -Wextra)
endif()

# Headless ABI test: every function of uison.h.
add_executable(abi-test abi-test/abi_test.c)
target_link_libraries(abi-test PRIVATE uison::uison)

# uison_kit.h test: C99 and C11 (field type checks via _Generic).
add_executable(kit-test abi-test/kit_test.c)
target_link_libraries(kit-test PRIVATE uison::uison)
add_executable(kit-test-c11 abi-test/kit_test.c)
target_link_libraries(kit-test-c11 PRIVATE uison::uison)
set_target_properties(kit-test-c11 PROPERTIES C_STANDARD 11)
if(NOT MSVC)
  target_compile_options(kit-test PRIVATE -pedantic)
  target_compile_options(kit-test-c11 PRIVATE -pedantic)
endif()

# form2 in C: the same form.json as the Rust example.
add_executable(form2-c form2-c/main.c)
target_link_libraries(form2-c PRIVATE uison::uison)
if(EMSCRIPTEN)
  uison_web_page(form2-c)
  uison_web_files(form2-c "${CMAKE_CURRENT_SOURCE_DIR}/../examples/form2/form.json")
  target_compile_definitions(form2-c PRIVATE FORM_JSON="form.json")
else()
  target_compile_definitions(form2-c PRIVATE FORM_JSON="${CMAKE_CURRENT_SOURCE_DIR}/../examples/form2/form.json")
endif()

# The low level: a custom frame of primitives.
add_executable(app-c app-c/main.c)
target_link_libraries(app-c PRIVATE uison::uison)
uison_web_page(app-c)

# ---------------------------------------------------------------- lessons (docs/c/lessons, tools/lessons.sh c)
# c-NN-name: the lesson program; the lesson's *.json files and images (ARGN, "path@name") go next to it.
# In the browser (emcmake): the page c-NN-name.html, files in its file system.
function(uison_lesson name src)
  add_executable(${name} ${src})
  target_link_libraries(${name} PRIVATE uison::uison)
  uison_web_page(${name})
  get_filename_component(dir "${src}" DIRECTORY)
  file(GLOB extra "${CMAKE_CURRENT_SOURCE_DIR}/${dir}/*.json")
  list(FILTER extra EXCLUDE REGEX "scenario\\.json$")
  uison_web_files(${name} ${extra} ${ARGN})
endfunction()

uison_lesson(c-01-hello lessons/01-hello/main.c)
uison_lesson(c-02-data lessons/02-data/main.c)
uison_lesson(c-03-forms lessons/03-forms/main.c)
uison_lesson(c-04-lists lessons/04-lists/main.c)
uison_lesson(c-05-files lessons/05-files/main.c)
uison_lesson(c-06-themes lessons/06-themes/main.c)
# Images for lesson 7 come from the gallery, the photo as photo.jpg.
uison_lesson(c-07-rich lessons/07-rich/main.c
  "${CMAKE_CURRENT_SOURCE_DIR}/../examples/gallery/images/anna.jpg@photo.jpg"
  "${CMAKE_CURRENT_SOURCE_DIR}/../examples/gallery/images/landscape.png")
# Threads: pthread / _beginthreadex and stdatomic (C11). There are no threads in the browser.
if(NOT EMSCRIPTEN)
  find_package(Threads REQUIRED)
  uison_lesson(c-08-threads lessons/08-threads/main.c)
  target_link_libraries(c-08-threads PRIVATE Threads::Threads)
  set_target_properties(c-08-threads PROPERTIES C_STANDARD 11)
endif()
uison_lesson(c-09-frame lessons/09-frame/main.c)
