# Folder of these files (uison-android.map is next to them).
set(UISON_CMAKE_DIR "${CMAKE_CURRENT_LIST_DIR}")

# System libraries the static libuison needs (rustc --print native-static-libs).
# Shared file: the package (uisonConfig.cmake) and the build from source (CMakeLists.txt).
if(IOS)
  set(UISON_SYSTEM_LIBS "-framework UIKit" "-framework QuartzCore" "-framework CoreGraphics" "-framework CoreText"
      "-framework ImageIO" "-framework CoreFoundation" "-framework Foundation" objc)
elseif(APPLE)
  set(UISON_SYSTEM_LIBS "-framework AppKit" "-framework CoreGraphics" "-framework CoreText" "-framework ImageIO"
      "-framework CoreFoundation" "-framework Foundation" objc)
elseif(ANDROID)
  set(UISON_SYSTEM_LIBS log android dl m)
elseif(EMSCRIPTEN)
  # Browser: all JS (canvas, input) lives inside libuison.a as EM_JS strings; no system libraries.
  set(UISON_SYSTEM_LIBS)
elseif(WIN32)
  set(UISON_SYSTEM_LIBS dxgi d2d1 d3d11 dwrite dwmapi user32 shell32 gdi32 winmm kernel32 advapi32 ole32 ntdll userenv ws2_32 dbghelp)
else()
  # Linux: Wayland, X11, Cairo, Pango — via dlopen; the binary depends only on libc.
  set(UISON_SYSTEM_LIBS gcc_s util rt pthread m dl c)
endif()

if(WIN32)
  set(UISON_STATIC_NAME uison.lib)
  set(UISON_SHARED_NAME uison.dll)
  set(UISON_IMPLIB_NAME uison.dll.lib)
elseif(APPLE)
  set(UISON_STATIC_NAME libuison.a)
  set(UISON_SHARED_NAME libuison.dylib)
else()
  set(UISON_STATIC_NAME libuison.a)
  set(UISON_SHARED_NAME libuison.so)
endif()

# Targets uison::uison (static) and uison::shared (dynamic). _lib — where the library files are,
# _bin — where the .dll is (Windows), _inc — headers.
function(uison_add_targets _lib _bin _inc)
  if(NOT TARGET uison::uison)
    add_library(uison_static INTERFACE)
    target_include_directories(uison_static INTERFACE "${_inc}")
    target_link_libraries(uison_static INTERFACE "${_lib}/${UISON_STATIC_NAME}" ${UISON_SYSTEM_LIBS})
    if(APPLE)
      # Without -dead_strip the whole library ends up in the program.
      target_link_options(uison_static INTERFACE "LINKER:-dead_strip")
    elseif(NOT WIN32)
      target_link_options(uison_static INTERFACE "LINKER:--gc-sections")
    endif()
    if(EMSCRIPTEN AND UISON_WASM_EXCEPTIONS)
      # uison.hpp reports errors with exceptions: WebAssembly exceptions (without them throw is abort).
      target_compile_options(uison_static INTERFACE $<$<COMPILE_LANGUAGE:CXX>:-fwasm-exceptions>)
      target_link_options(uison_static INTERFACE -fwasm-exceptions)
    endif()
    if(ANDROID)
      # The program is libmain.so: the JNI_OnLoad entry point (from libuison) calls its main().
      target_link_options(uison_static INTERFACE "LINKER:-u,JNI_OnLoad" "LINKER:--version-script=${UISON_CMAKE_DIR}/uison-android.map")
    endif()
    add_library(uison::uison ALIAS uison_static)
  endif()
  # iOS and the browser: the app is a single program, no dynamic library.
  if(NOT TARGET uison::shared AND NOT IOS AND NOT EMSCRIPTEN)
    add_library(uison_shared_lib SHARED IMPORTED GLOBAL)
    if(WIN32)
      set_target_properties(uison_shared_lib PROPERTIES
        IMPORTED_LOCATION "${_bin}/${UISON_SHARED_NAME}" IMPORTED_IMPLIB "${_lib}/${UISON_IMPLIB_NAME}")
    else()
      set_target_properties(uison_shared_lib PROPERTIES IMPORTED_LOCATION "${_lib}/${UISON_SHARED_NAME}")
    endif()
    add_library(uison_shared INTERFACE)
    target_include_directories(uison_shared INTERFACE "${_inc}")
    target_link_libraries(uison_shared INTERFACE uison_shared_lib)
    if(WIN32)
      target_compile_definitions(uison_shared INTERFACE UISON_DLL)
    endif()
    add_library(uison::shared ALIAS uison_shared)
  endif()
endfunction()

# Windows: copy uison.dll next to the program after the build (for uison::shared; other systems use rpath).
function(uison_copy_dll target)
  if(WIN32)
    add_custom_command(TARGET ${target} POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:uison_shared_lib> $<TARGET_FILE_DIR:${target}>)
  endif()
endfunction()

# C++ widgets for your own components (like uison_widgets.hpp, but for the program's components.json):
#   uison_generate_widgets(app components.json [NAMESPACE app] [OUTPUT app_widgets.hpp])
#   → #include "app_widgets.hpp"; using namespace app;  Card({.title = "…"})
# The header is regenerated when components.json changes. The generator is uison-gen (UISON_GEN):
# built by cargo from source, bin/uison-gen in the package.
function(uison_generate_widgets target json)
  cmake_parse_arguments(G "" "NAMESPACE;OUTPUT" "" ${ARGN})
  if(NOT G_NAMESPACE)
    set(G_NAMESPACE app)
  endif()
  if(NOT G_OUTPUT)
    set(G_OUTPUT "${G_NAMESPACE}_widgets.hpp")
  endif()
  if(NOT UISON_GEN)
    message(FATAL_ERROR "uison_generate_widgets: no uison-gen (set UISON_GEN or put uison-gen in PATH)")
  endif()
  get_filename_component(json "${json}" ABSOLUTE)
  set(dir "${CMAKE_CURRENT_BINARY_DIR}/uison-gen/${target}")
  set(out "${dir}/${G_OUTPUT}")
  file(MAKE_DIRECTORY "${dir}")
  add_custom_command(OUTPUT "${out}"
    COMMAND "${UISON_GEN}" --namespace "${G_NAMESPACE}" -o "${out}" "${json}"
    DEPENDS "${json}" ${UISON_GEN_DEPENDS}
    COMMENT "uison-gen: ${G_OUTPUT}"
    VERBATIM)
  target_sources(${target} PRIVATE "${out}")
  target_include_directories(${target} PRIVATE "${dir}")
endfunction()

# Browser (Emscripten). Turn UISON_WASM_EXCEPTIONS off if the program is built without exceptions
# or with -fexceptions (exceptions via JS).
option(UISON_WASM_EXCEPTIONS "Emscripten: link uison::uison with -fwasm-exceptions" ON)

# The program as a page: target.html + target.js + target.wasm (full-window canvas, template web/shell.html).
function(uison_web_page target)
  if(EMSCRIPTEN)
    set_target_properties(${target} PROPERTIES SUFFIX ".html")
    target_link_options(${target} PRIVATE "--shell-file=${UISON_WEB_DIR}/shell.html")
    set_property(TARGET ${target} APPEND PROPERTY LINK_DEPENDS "${UISON_WEB_DIR}/shell.html")
  endif()
endfunction()

# Files next to the program (JSON screen, images): into the Emscripten file system under their own name
# (the program reads them: uison_ui_load, image_file) and next to the page (the browser loads images by URL).
# On other systems — a copy next to the program. "path@name" — under a different name.
function(uison_web_files target)
  foreach(f ${ARGN})
    get_filename_component(name "${f}" NAME)
    if(f MATCHES "^(.+)@([^@/]+)$")
      set(f "${CMAKE_MATCH_1}")
      set(name "${CMAKE_MATCH_2}")
    endif()
    if(EMSCRIPTEN)
      target_link_options(${target} PRIVATE "SHELL:--preload-file \"${f}@/${name}\"")
      set_property(TARGET ${target} APPEND PROPERTY LINK_DEPENDS "${f}")
    endif()
    add_custom_command(TARGET ${target} POST_BUILD COMMAND ${CMAKE_COMMAND} -E copy_if_different "${f}" "$<TARGET_FILE_DIR:${target}>/${name}")
  endforeach()
endfunction()
