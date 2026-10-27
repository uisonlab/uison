# Урок 11. Везде

[English](../../../rust/lessons/11-everywhere.md) · **Русский**

**Что будет:** та же программа — свой крейт из [быстрого старта](../quickstart.md) — на iPhone, на Android и в браузере. Код не меняется: `uison::run` на каждой платформе свой.

Упаковку (`.app`, APK, страница) делают скрипты из репозитория uison — им достаточно готовой программы. Ниже `UISON` — путь к клону репозитория, `hello` — ваш крейт.

## Браузер

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
$UISON/tools/web.sh target/wasm32-unknown-unknown/release/hello.wasm     # → target/web/hello/
python3 $UISON/tools/serve.py target/web/hello 8000                        # http://localhost:8000/
```

Страница — `index.html`, `uison.js` (рисование на canvas, ввод) и `app.wasm`. Без wasm-bindgen и без обвязки: программа та же, `main` вызывается из `uison.js`. Подробно — [браузер](../../wasm.md).

## iPhone и iPad

Нужен Xcode (симулятор) — и для телефона подпись разработчика ([телефоны](../../mobile.md#ios)).

```sh
rustup target add aarch64-apple-ios-sim aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
$UISON/tools/ios-run.sh target/aarch64-apple-ios-sim/release/hello sim   # .app → simulator; output goes here
```

Со сценарием проверки и снимком — одной командой: `$UISON/tools/ios-test.sh target/aarch64-apple-ios-sim/release/hello scenario.json sim`. Для телефона — `--target aarch64-apple-ios` и `device`.

## Android

Нужны Android SDK и NDK ([телефоны](../../mobile.md#android)). Программа на Android — библиотека `libmain.so`: Java-прослойка uison загружает её и вызывает `main`.

```sh
rustup target add aarch64-linux-android
NDK=~/Library/Android/sdk/ndk/<version>   # on Linux — ~/Android/Sdk/ndk/<version>
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android29-clang
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-shared"
cargo build --release --target aarch64-linux-android
mkdir -p hello/arm64-v8a && cp target/aarch64-linux-android/release/hello hello/arm64-v8a/libmain.so
$UISON/tools/android-run.sh hello                                         # APK → phone, launch
```

Имя приложения — по папке (`hello`). Со сценарием и снимком — `$UISON/tools/android-test.sh hello scenario.json`.

## Что меняется на телефоне

Ничего в коде, но экран другой — и это видно в данных `@env`:
- `@env.touch` — касания вместо мыши: тап, прокрутка пальцем с инерцией, долгое нажатие вместо правой кнопки;
- безопасная зона (вырез камеры, полоса «домой») — корень экрана по умолчанию внутри неё (`@env.safe.*`);
- экранная клавиатура поднимается, когда фокус в поле, и раскладка сжимается над ней;
- размер — экран целиком: `size` из экрана не действует, а `@env.width` говорит, узкий ли он: `visible: { lt: ["@env.width", 600] }`.

## Иконка программы

Иконка работающего окна (Dock на macOS, заголовок и панель задач на Windows, панель задач на X11) — PNG, который задаётся до `run`:

```rust
uison::set_icon(include_bytes!("../assets/icon.png")); // 256×256 is enough
run(ui, "Hello");
```

Иконка самого файла программы — дело упаковки. На macOS это пакет `.app` с `.icns`; на Windows — ресурс в `.exe` (его пишет `build.rs`); на Linux — файл `.desktop` и иконка в теме `hicolor` (на Wayland окно находит её по своему `app_id`, `dev.uison.<программа>`). Все три сделаны в примере Disk Usage: [`build.rs`](../../../../examples/disk-usage/build.rs), [`bundle.sh`](../../../../examples/disk-usage/bundle.sh).

Конец серии. Справочник — [язык экранов](../../screens/README.md), [элементы](../../screens/elements.md).
