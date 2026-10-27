# C++ examples (C ones are in examples-c). By default the library is built from source (needs Rust);
# -DUISON_PACKAGE=path/to/dist/uison-<platform> uses a prebuilt package (tools/c-dist.sh).
#   cmake -S examples-cpp -B out/cpp/build && cmake --build out/cpp/build
cmake_minimum_required(VERSION 3.19)
project(uison_examples_cpp LANGUAGES C CXX)

set(UISON_PACKAGE "" CACHE PATH "Prebuilt uison package (dist/uison-<platform>); empty: build from source")
if(UISON_PACKAGE)
  list(APPEND CMAKE_PREFIX_PATH "${UISON_PACKAGE}")
  # The iOS, NDK and Emscripten toolchains look for packages only in their sysroot, so the path is explicit.
  set(uison_DIR "${UISON_PACKAGE}/lib/cmake/uison")
  find_package(uison REQUIRED)
else()
  add_subdirectory(../crates/uison-c uison)
endif()

set(CMAKE_CXX_STANDARD 20)
if(MSVC)
  add_compile_options(/W4 /utf-8)
else()
  add_compile_options(-Wall -Wextra)
endif()

# Headless uison.hpp test: conversions, struct reflection, the builder, lambdas;
# the same test with the shared library.
add_executable(cpp-test cpp-test/cpp_test.cpp)
target_link_libraries(cpp-test PRIVATE uison::uison)
# The canonical event form shared with Rust (scenarios/events.jsonl).
target_compile_definitions(cpp-test PRIVATE UISON_EVENTS_FILE="${CMAKE_CURRENT_SOURCE_DIR}/../scenarios/events.jsonl")
# Handlers referencing main's locals ([&]): in the browser main is not unwound (scenario locals.json).
add_executable(cpp-locals cpp-test/locals.cpp)
target_link_libraries(cpp-locals PRIVATE uison::uison)
uison_web_page(cpp-locals)
if(TARGET uison::shared)
  add_executable(cpp-test-shared cpp-test/cpp_test.cpp)
  target_link_libraries(cpp-test-shared PRIVATE uison::shared)
  target_compile_definitions(cpp-test-shared PRIVATE UISON_EVENTS_FILE="${CMAKE_CURRENT_SOURCE_DIR}/../scenarios/events.jsonl")
  uison_copy_dll(cpp-test-shared)
endif()

# A todo list: the screen builder, structs <-> data (C++20).
add_executable(todo-cpp todo-cpp/main.cpp)
target_link_libraries(todo-cpp PRIVATE uison::uison)
uison_web_page(todo-cpp)

# A virtual list of 10,000 rows (--bench measures data transfer); photos come from the gallery.
add_executable(list10k-cpp list10k-cpp/main.cpp)
target_link_libraries(list10k-cpp PRIVATE uison::uison)
uison_web_page(list10k-cpp)
foreach(n anna boris vera gleb)
  uison_web_files(list10k-cpp "${CMAKE_CURRENT_SOURCE_DIR}/../examples/gallery/images/${n}.jpg")
endforeach()

# A worker thread and progress: uison::Handle::post. There are no threads in the browser (docs/wasm.md).
if(NOT EMSCRIPTEN)
  find_package(Threads REQUIRED)
  add_executable(worker-cpp worker-cpp/main.cpp)
  target_link_libraries(worker-cpp PRIVATE uison::uison Threads::Threads)
endif()

# ---------------------------------------------------------------- lessons (docs/cpp/lessons, tools/lessons.sh cpp)
# cpp-NN-name: the lesson program; the lesson's *.json files and images (ARGN, "path@name") go next to it.
# In the browser (emcmake): the page cpp-NN-name.html, files in its file system.
function(uison_lesson name src)
  add_executable(${name} ${src})
  target_link_libraries(${name} PRIVATE uison::uison)
  uison_web_page(${name})
  get_filename_component(dir "${src}" DIRECTORY)
  file(GLOB extra "${CMAKE_CURRENT_SOURCE_DIR}/${dir}/*.json")
  list(FILTER extra EXCLUDE REGEX "scenario\\.json$")
  uison_web_files(${name} ${extra} ${ARGN})
endfunction()

uison_lesson(cpp-01-hello lessons/01-hello/main.cpp)
uison_lesson(cpp-02-data lessons/02-data/main.cpp)
uison_lesson(cpp-03-forms lessons/03-forms/main.cpp)
uison_lesson(cpp-04-lists lessons/04-lists/main.cpp)
uison_lesson(cpp-05-structs lessons/05-structs/main.cpp)
# Custom components: uison-gen generates C++ widgets from components.json (app_widgets.hpp).
uison_lesson(cpp-06-components lessons/06-components/main.cpp)
uison_generate_widgets(cpp-06-components lessons/06-components/components.json)
uison_lesson(cpp-07-themes lessons/07-themes/main.cpp)
# Images for lesson 8 come from the gallery, the photo as photo.jpg.
uison_lesson(cpp-08-rich lessons/08-rich/main.cpp
  "${CMAKE_CURRENT_SOURCE_DIR}/../examples/gallery/images/anna.jpg@photo.jpg"
  "${CMAKE_CURRENT_SOURCE_DIR}/../examples/gallery/images/landscape.png")
# Threads: std::jthread (C++20). There are no threads in the browser.
# libc++ before LLVM 20 (Xcode 16 and older) has jthread only with -fexperimental-library.
if(NOT EMSCRIPTEN)
  include(CheckCXXSourceCompiles)
  set(jthread_test "#include <thread>
int main() { std::jthread t([](std::stop_token s) { (void)s.stop_requested(); }); t.request_stop(); }")
  set(CMAKE_REQUIRED_LIBRARIES Threads::Threads)
  check_cxx_source_compiles("${jthread_test}" UISON_HAVE_JTHREAD)
  if(NOT UISON_HAVE_JTHREAD)
    set(CMAKE_REQUIRED_FLAGS -fexperimental-library)
    check_cxx_source_compiles("${jthread_test}" UISON_HAVE_JTHREAD_EXPERIMENTAL)
    unset(CMAKE_REQUIRED_FLAGS)
  endif()
  unset(CMAKE_REQUIRED_LIBRARIES)
  if(UISON_HAVE_JTHREAD OR UISON_HAVE_JTHREAD_EXPERIMENTAL)
    uison_lesson(cpp-09-threads lessons/09-threads/main.cpp)
    target_link_libraries(cpp-09-threads PRIVATE Threads::Threads)
    if(NOT UISON_HAVE_JTHREAD)
      target_compile_options(cpp-09-threads PRIVATE -fexperimental-library)
      target_link_options(cpp-09-threads PRIVATE -fexperimental-library)
    endif()
  else()
    message(STATUS "cpp-09-threads skipped: the C++ library has no std::jthread")
  endif()
endif()
uison_lesson(cpp-10-frame lessons/10-frame/main.cpp)
# C++17: a method chain and a JSON string, without designated initializers.
uison_lesson(cpp-11-cpp17 lessons/11-cpp17/main.cpp)
set_target_properties(cpp-11-cpp17 PROPERTIES CXX_STANDARD 17)
