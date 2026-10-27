# find_package(uison) from a prebuilt package (tools/c-dist.sh):
#   target_link_libraries(app PRIVATE uison::uison)      # static — a single program with no dependencies
#   target_link_libraries(app PRIVATE uison::shared)   # dynamic — libuison.dylib / .so / uison.dll next to it
get_filename_component(_uison_root "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
include("${CMAKE_CURRENT_LIST_DIR}/uison-libs.cmake")
# iOS (inside uison.xcframework) and Android packages — one folder per platform/ABI.
set(_uison_lib "${_uison_root}/lib")
if(IOS)
  if(CMAKE_OSX_SYSROOT MATCHES "[Ss]imulator")
    set(_uison_lib "${_uison_root}/uison.xcframework/ios-arm64-simulator")
  else()
    set(_uison_lib "${_uison_root}/uison.xcframework/ios-arm64")
  endif()
elseif(ANDROID)
  set(_uison_lib "${_uison_lib}/${CMAKE_ANDROID_ARCH_ABI}")
endif()
set(UISON_WEB_DIR "${_uison_root}/web" CACHE INTERNAL "uison: shell.html for Emscripten pages")
# Widget generator for your own components (uison_generate_widgets): from the package, otherwise from PATH.
find_program(UISON_GEN uison-gen HINTS "${_uison_root}/bin" NO_CMAKE_FIND_ROOT_PATH)
uison_add_targets("${_uison_lib}" "${_uison_root}/bin" "${_uison_root}/include")
set(uison_FOUND TRUE)
