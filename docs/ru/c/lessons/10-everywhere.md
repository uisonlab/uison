# Урок 10. Везде

[English](../../../c/lessons/10-everywhere.md) · **Русский**

**Что будет:** программа из урока 1 — на iPhone, на Android и в браузере. Код не меняется: `uison_run` на каждой платформе свой.

## Браузер

[Emscripten](https://emscripten.org) собирает программу на C в WebAssembly привычным способом — `emcc` или `emcmake`. Нужен пакет `dist/uison-wasm` (`tools/c-dist.sh wasm`):

```sh
D=dist/uison-wasm
emcc -std=c11 -Oz -I$D/include main.c $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
python3 tools/serve.py site 8000          # http://localhost:8000/
```

Других флагов не нужно: JS рисования и ввода лежит внутри `libuison.a`. Через CMake — `emcmake cmake …` и `uison_web_page(app)`; файлы рядом с программой (урок 5) — `uison_web_files(app screen.json …)`. Подробно — [браузер](../../wasm.md).

## iPhone и iPad

```sh
D=dist/uison-ios                          # tools/c-dist.sh ios
clang -target arm64-apple-ios17.0-simulator -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) \
  main.c -I$D/include $D/uison.xcframework/ios-arm64-simulator/libuison.a \
  -framework UIKit -framework QuartzCore -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello
tools/ios-run.sh ./hello sim                                      # .app → simulator; output goes here
tools/ios-test.sh ./hello scenarios/lessons/hello.json sim        # scenario and screenshot
```

Для телефона — `-target arm64-apple-ios17.0`, `--sdk iphoneos`, `uison.xcframework/ios-arm64` и `device` (нужна подпись). Файлы рядом с программой (урок 5) — `UISON_IOS_FILES="screen.json components.json"`. CMake, Xcode — [телефоны](../../mobile.md#ios).

## Android

Программа на Android — библиотека `libmain.so`: Java-прослойка uison загружает её, а `JNI_OnLoad` из `libuison.a` вызывает ваш `main`.

```sh
D=dist/uison-android                      # tools/c-dist.sh android
NDK=~/Library/Android/sdk/ndk/<version>; CC=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin   # Linux: linux-x86_64
mkdir -p hello/arm64-v8a
$CC/aarch64-linux-android29-clang -shared -fPIC main.c -I$D/include $D/lib/arm64-v8a/libuison.a \
  -llog -landroid -ldl -lm -Wl,--gc-sections,--version-script=$D/lib/cmake/uison/uison-android.map -s \
  -o hello/arm64-v8a/libmain.so
tools/android-run.sh hello                                        # APK → phone, launch
tools/android-test.sh hello scenarios/lessons/hello.json          # scenario and screenshot
```

Имя приложения — по папке (`hello`). Файлы рядом с программой — `UISON_ANDROID_ASSETS="screen.json components.json"`. Второй ABI (`armeabi-v7a`), CMake с тулчейном NDK — [телефоны](../../mobile.md#android).

## Что меняется на телефоне

Ничего в коде, но экран другой — и это видно в данных `@env`: касания вместо мыши (`@env.touch`), безопасная зона под вырезом и полосой «домой», экранная клавиатура (раскладка сжимается над ней), размер — экран целиком. Узкий экран — `visible: { lt: ["@env.width", 600] }`.

## Иконка программы

Иконка работающего окна (Dock на macOS, заголовок и панель задач на Windows, панель задач на X11) — PNG, который задаётся до `uison_run`:

```c
#include "icon.h" /* xxd -i icon.png > icon.h: unsigned char icon_png[], unsigned int icon_png_len */
uison_set_icon(icon_png, icon_png_len);
return uison_run(ui, "Hello");
```

Иконка самого файла программы — дело упаковки. На Windows в исходники программы добавляется сценарий ресурсов — `add_executable(app main.c app.rc)`, где `app.rc` — одна строка `1 ICON "icon.ico"` (номер 1 берёт окно); на macOS — пакет `.app` с `.icns`; на Linux — файл `.desktop` и иконка в теме `hicolor` (на Wayland окно находит её по своему `app_id`, `dev.uison.<программа>`). Готовый пример для macOS и Linux — [`bundle.sh`](../../../../examples/disk-usage/bundle.sh) из Disk Usage.

Конец серии. Справочник — [язык экранов](../../screens/README.md), [элементы](../../screens/elements.md), API — [`uison.h`](../../../../crates/uison-c/include/uison.h).
