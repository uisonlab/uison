# Building from source inside your own CMake project:
#   add_subdirectory(path/to/uison/crates/uison-c uison)
#   target_link_libraries(app PRIVATE uison::uison)      # or uison::shared
# CMake calls cargo (Rust required: https://rustup.rs) and puts the library into target/release.
cmake_minimum_required(VERSION 3.19)
project(uison LANGUAGES C)

find_program(UISON_CARGO cargo HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin" REQUIRED)
get_filename_component(_uison_ws "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)
# UISON_CARGO_TARGET — the Rust target (e.g. aarch64-apple-ios); defaults to the current system.
# The target is always explicit: builds for different systems from one folder (a folder shared with
# a Linux machine, Docker) don't overwrite each other in target/release.
find_program(UISON_RUSTC rustc HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin" REQUIRED)
if(IOS)
  if(CMAKE_OSX_SYSROOT MATCHES "[Ss]imulator")
    set(_uison_triple aarch64-apple-ios-sim)
  else()
    set(_uison_triple aarch64-apple-ios)
  endif()
elseif(ANDROID)
  set(_uison_abis_arm64-v8a aarch64-linux-android)
  set(_uison_abis_armeabi-v7a armv7-linux-androideabi)
  set(_uison_abis_x86_64 x86_64-linux-android)
  set(_uison_abis_x86 i686-linux-android)
  set(_uison_triple ${_uison_abis_${CMAKE_ANDROID_ARCH_ABI}})
elseif(EMSCRIPTEN)
  set(_uison_triple wasm32-unknown-emscripten)
else()
  execute_process(COMMAND "${UISON_RUSTC}" -vV OUTPUT_VARIABLE _uison_rustc_v)
  string(REGEX MATCH "host: ([^\n]+)" _ "${_uison_rustc_v}")
  set(_uison_triple "${CMAKE_MATCH_1}")
endif()
set(UISON_CARGO_TARGET "${_uison_triple}" CACHE STRING "Rust target triple for uison")
set(_uison_out "${_uison_ws}/target/${UISON_CARGO_TARGET}/release")
set(_uison_target_arg --target ${UISON_CARGO_TARGET})
# cargo environment: minimum iOS version; the NDK linker for libuison.so on Android.
set(_uison_env --unset=MAKEFLAGS --unset=MFLAGS)
if(IOS AND CMAKE_OSX_DEPLOYMENT_TARGET)
  list(APPEND _uison_env "IPHONEOS_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}")
elseif(ANDROID)
  string(TOUPPER "${UISON_CARGO_TARGET}" _uison_t)
  string(REPLACE "-" "_" _uison_t "${_uison_t}")
  list(APPEND _uison_env "CARGO_TARGET_${_uison_t}_LINKER=${CMAKE_C_COMPILER}"
       "CARGO_TARGET_${_uison_t}_RUSTFLAGS=-Clink-arg=--target=${CMAKE_C_COMPILER_TARGET}")
endif()

include("${CMAKE_CURRENT_SOURCE_DIR}/cmake/uison-libs.cmake")
set(UISON_WEB_DIR "${_uison_ws}/web" CACHE INTERNAL "uison: shell.html for Emscripten pages")
# Emscripten: static only (a cdylib can't be built for wasm32-unknown-emscripten); no linker needed —
# emcc links the program.
set(_uison_build build --release -p uison-c ${_uison_target_arg})
set(_uison_byproducts "${_uison_out}/${UISON_STATIC_NAME}" "${_uison_out}/${UISON_SHARED_NAME}")
if(EMSCRIPTEN)
  set(_uison_build rustc --release -p uison-c ${_uison_target_arg} --crate-type staticlib)
  set(_uison_byproducts "${_uison_out}/${UISON_STATIC_NAME}")
endif()
# Strip the dynamic library ourselves (rustc's strip breaks the .dylib for Apple's ld).
if(APPLE AND NOT IOS)
  set(_uison_strip COMMAND strip -x "${_uison_out}/${UISON_SHARED_NAME}")
elseif(NOT WIN32 AND NOT IOS AND NOT EMSCRIPTEN AND CMAKE_STRIP)
  set(_uison_strip COMMAND "${CMAKE_STRIP}" --strip-unneeded "${_uison_out}/${UISON_SHARED_NAME}")
endif()
add_custom_target(uison_cargo
  # cargo doesn't need make's MAKEFLAGS (otherwise it warns about the jobserver).
  COMMAND "${CMAKE_COMMAND}" -E env ${_uison_env} "${UISON_CARGO}" ${_uison_build}
  ${_uison_strip}
  WORKING_DIRECTORY "${_uison_ws}"
  BYPRODUCTS ${_uison_byproducts}
  COMMENT "cargo build -p uison-c (libuison)"
  USES_TERMINAL)
uison_add_targets("${_uison_out}" "${_uison_out}" "${CMAKE_CURRENT_SOURCE_DIR}/include")

# uison-gen for uison_generate_widgets — a program for the build machine (not the target: iOS, Android, browser).
execute_process(COMMAND "${UISON_RUSTC}" -vV OUTPUT_VARIABLE _uison_host_v)
string(REGEX MATCH "host: ([^\n]+)" _ "${_uison_host_v}")
set(_uison_host "${CMAKE_MATCH_1}")
set(_uison_gen_exe "")
if(CMAKE_HOST_WIN32)
  set(_uison_gen_exe ".exe")
endif()
set(_uison_gen "${_uison_ws}/target/${_uison_host}/release/uison-gen${_uison_gen_exe}")
add_custom_command(OUTPUT "${_uison_gen}"
  COMMAND "${CMAKE_COMMAND}" -E env ${_uison_env} "${UISON_CARGO}" build --release -p uison-gen --target ${_uison_host}
  WORKING_DIRECTORY "${_uison_ws}"
  DEPENDS "${_uison_ws}/crates/uison-gen/src/lib.rs" "${_uison_ws}/crates/uison-gen/src/main.rs"
  COMMENT "cargo build -p uison-gen"
  USES_TERMINAL)
add_custom_target(uison_gen_tool DEPENDS "${_uison_gen}")
set(UISON_GEN "${_uison_gen}" CACHE INTERNAL "uison: widgets generator")
set(UISON_GEN_DEPENDS uison_gen_tool CACHE INTERNAL "")
add_dependencies(uison_static uison_cargo)
if(TARGET uison_shared)
  add_dependencies(uison_shared uison_cargo)
endif()
