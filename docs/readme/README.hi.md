# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<b>🇮🇳 हिन्दी</b> ·
<a href="README.es.md">🇪🇸 Español</a> ·
<a href="README.fr.md">🇫🇷 Français</a> ·
<a href="README.ar.md">🇸🇦 العربية</a> ·
<a href="README.bn.md">🇧🇩 বাংলা</a> ·
<a href="README.pt.md">🇧🇷 Português</a> ·
<a href="README.ru.md">🇷🇺 Русский</a> ·
<a href="README.ur.md">🇵🇰 اردو</a> ·
<a href="README.id.md">🇮🇩 Bahasa Indonesia</a> ·
<a href="README.de.md">🇩🇪 Deutsch</a> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) Rust में लिखा गया एक छोटा और तेज़ UI फ़्रेमवर्क है, जिसकी कोई भी dependency
नहीं है — सिर्फ़ standard library, यहाँ तक कि JSON parser भी अपना है। आप स्क्रीन को elements के एक
tree के रूप में बताते हैं: JSON फ़ाइल में, Rust DSL से, Flutter जैसे C++ widgets से, या सीधे C कोड के
अंदर JSON लिखकर। लॉजिक Rust, C या C++ का साधारण कोड होता है: handlers, डेटा, threads। window, layout,
drawing और input की ज़िम्मेदारी uison की है, और वही प्रोग्राम macOS, Windows, Linux (Wayland और X11),
iOS, Android और ब्राउज़र में चलता है।

हर backend हाथ से लिखे FFI के ज़रिए सीधे सिस्टम से बात करता है: AppKit और CoreText, Direct2D और
DirectWrite के साथ Win32 (COM भी हाथ से), शुरू से लिखा गया Wayland protocol और X11, UIKit, JNI के
ज़रिए Android का Canvas, और WebAssembly से 2D canvas। release प्रोग्राम लगभग 0.7–0.9 MB की एक ही फ़ाइल
होता है, जिसे सिस्टम की अपनी libraries के अलावा कुछ नहीं चाहिए, और desktop पर कोई event लगभग एक
मिलीसेकंड में स्क्रीन तक पहुँच जाता है।

![uison से बने प्रोग्राम: Studio (मेल, सेटिंग्स, चार्ट), एलिमेंट गैलरी, Todo, Disk Usage और एक छोटा वेब ब्राउज़र](../img/showcase.jpg)

## यह कैसे काम करता है

```text
स्क्रीन (JSON, Rust DSL, C++ widgets, C में UISON_JSON)  +  डेटा  +  आपकी भाषा में handlers
        │  bindings, templates, themes, layout (uison-ui)
        ▼
primitives का frame: rect, line, circle, tri, poly, text, image, clip, hit
        │  पिछले frame से diff → सिर्फ़ बदला हुआ हिस्सा फिर से बनाना
        ▼
सिस्टम backend: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

स्क्रीन एक *frame* में compile होती है: कुछ ही तरह के primitives की एक सपाट सूची — rectangles,
lines, circles, polygons, text की एक-एक पंक्ति, images, clip regions और mouse व touch के लिए अदृश्य hit
areas। हर नए frame की तुलना पिछले frame से की जाती है, और backend सिर्फ़ वही regions दोबारा बनाता है जो
बदले हैं। backend छोटा इसलिए है क्योंकि उसे बस ये primitives बनाने, text नापने और सिस्टम के events को
uison के events में बदलने होते हैं।

स्क्रीन की भाषा में कोई कोड नहीं होता। text और properties डेटा से bind होते हैं (`"Pressed {@n} times"`,
`"disabled": "!@form.valid"`), elements parameters वाले दोबारा इस्तेमाल होने वाले templates हैं, रंग और
आकार theme tokens से आते हैं, और events नाम वाले actions को बुलाते हैं, जिनके handlers Rust, C या C++
में लिखे जाते हैं। जब कोई handler डेटा बदलता है, तो स्क्रीन अपने आप फिर से बन जाती है।

## Hello, world

एक ही window — एक counter और "+1" बटन — तीनों भाषाओं में।

**Rust** — स्क्रीन DSL से बनती है, handler बटन पर एक closure है:

```rust
use uison::prelude::*;

fn main() {
    let screen = center(column().gap(12).cross("center").children([
        text("Pressed {@n} times").size(20),
        button("+1").key("inc").on_click(|cx| cx.data.edit("n", |n: &mut i32| *n += 1)),
    ]));
    let mut ui = Ui::from_node(screen).with_size(320.0, 200.0);
    ui.data.set("n", 0);
    run(ui, "Hello");
}
```

**C** — स्क्रीन सीधे कोड में लिखा JSON है, handler एक function है:

```c
#include <uison_kit.h>

static void inc(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [320, 200],
        data: { n: 0 },
        root: { Center: { child: { Column: { gap: 12, cross: "center", children: [
            { Text: { text: "Pressed {@n} times", size: 20 } },
            { Button: { key: "inc", text: "+1", on: { click: "inc" } } },
        ] } } } }
    }));
    if (!ui) return uison_kit_fail(); /* prints the JSON error and where it is */
    UISON_ON(ui, inc);                /* action "inc" → function inc */
    return uison_run(ui, "Hello");
}
```

**C++** — Flutter जैसे widgets, handler बटन पर एक lambda है:

```cpp
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Hello");
}
```

अपना प्रोजेक्ट शुरू करने के लिए [Rust](../rust/quickstart.md), [C](../c/quickstart.md) या
[C++](../cpp/quickstart.md) का quick start देखें।

## विशेषताएँ

- **कोई dependency नहीं।** core, JSON parser, layout, Wayland client, Linux settings portal के लिए
  D-Bus client और Windows के लिए COM bindings — सब यहीं `std` के ऊपर लिखे गए हैं। `Cargo.lock` में
  सिर्फ़ workspace के अपने crates हैं।
- **एक प्रोग्राम, छह प्लेटफ़ॉर्म।** macOS, Windows (x64 और ARM64), Linux (Wayland और X11, arm64 और
  x86_64), iOS, Android (arm64-v8a और armeabi-v7a) और ब्राउज़र (WebAssembly)। फ़ोन पर वही कोड touch,
  safe area, on-screen keyboard और inertial scrolling पा लेता है।
- **Partial redraw।** frames का diff निकाला जाता है और सिर्फ़ प्रभावित regions दोबारा बनते हैं; पतली
  lines हर backend पर एक ही तरह से physical pixels पर align होती हैं।
- **Elements की library**: buttons, icon buttons और 25 icons, checkboxes, radio buttons, switches,
  sliders, selects, tabs, selection और clipboard वाले text fields, progress bars, cards, list tiles,
  badges, avatars, tooltips, context menus, links और images। elements खुद JSON templates
  (`lib/*.json`) हैं, और आपके अपने components भी इसी तरह लिखे जाते हैं।
- **Flutter जैसा layout**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, scrolling, हज़ारों-हज़ार
  rows के लिए virtualized lists, और `Flow` — inline text runs का एक paragraph, जिसमें wrapping उनके पार
  होती है।
- **Bindings और state**: डेटा के paths (`@user.name`), text interpolation, component का local state
  (`@~open`), state के अनुसार selectors (hover, pressed, focus), `switch`, और कुछ functions (`eq`,
  `not`, `len`, `ratio`…)। structs डेटा में और वापस map होते हैं: Rust में `#[derive(Data)]`, C में
  field tables, C++20 में बिना macros के reflection।
- **Themes**: light, dark और "सिस्टम के अनुसार", ऐसे tokens से बनी जिन्हें आप override कर सकते हैं।
- **अपनी drawing**: `Canvas` node आपका function बुलाता है, जो उसके rectangle में primitives लिखता है,
  साधारण elements के बगल में।
- **Threads**: worker thread नतीजे वापस UI thread को भेजता है (`post`), ब्राउज़र को छोड़कर हर प्लेटफ़ॉर्म
  पर।
- **Live reload** debug builds में: `UISON_RELOAD=folder` के साथ स्क्रीन की JSON फ़ाइलें save करते ही
  दोबारा पढ़ी जाती हैं।
- **बिना window के testing**: एक scenario (`UISON_SCRIPT=scenario.json`) element के `key` से tap,
  typing, scrolling और screenshots करता है; वही scenarios हर प्लेटफ़ॉर्म पर, headless Chrome में, iOS
  simulator में और फ़ोन पर चलते हैं।
- **C और C++ API**: `uison.h` (C99), `uison_kit.h` (C के लिए helpers), `uison.hpp` (C++17, C++20 में
  और सुविधाएँ) और `uison_widgets.hpp` (Rust DSL वाले ही schema से generate होने वाले C++20 widgets);
  static और dynamic libraries, एक CMake package और `pkg-config`। C और C++ Emscripten से ब्राउज़र के
  लिए भी build होते हैं: `em++ main.cpp libuison.a -o index.html`।

## प्लेटफ़ॉर्म

| प्लेटफ़ॉर्म | backend | कहाँ जाँचा गया |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO, Objective-C runtime के ज़रिए | Apple Silicon Mac — मुख्य development मशीन |
| iOS | UIKit, CADisplayLink, macOS वाला ही CoreGraphics/CoreText कोड | simulator और iPhone 12 mini |
| Android | हाथ से लिखा JNI, `android.graphics.Canvas`, ~300 पंक्तियों की Java layer; APK बिना Gradle के build | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite और WIC, COM हाथ से; per-monitor DPI, dark title bar | Windows 11 PC, 100/125/150 % पर, और अलग-अलग scale वाले दो monitors |
| Linux | Wayland protocol का अपना कोड और X11 (xcb), Cairo + Pango, xkbcommon — सब `dlopen` के ज़रिए, इसलिए binary सिर्फ़ libc पर निर्भर है | headless Mutter, KWin, Sway, Weston और X11 (arm64 और x86_64); virtual machine में GNOME वाला Ubuntu 24.04 |
| ब्राउज़र | wasm-bindgen के बिना `wasm32-unknown-unknown`, canvas 2D; C/C++ Emscripten के ज़रिए | Chrome (और headless Chrome) |

अभी तक नहीं जाँचा गया: Windows पर असली touchscreen, Windows on ARM (build होता है पर अभी चलाया नहीं
गया), असली hardware पर Linux और KDE, GNOME में mouse और keyboard input, Safari और Firefox।

## आकार और गति

Release builds (`opt-level = "s"`, LTO, `panic = "abort"`, stripped)। आकार प्रोजेक्ट के बढ़ने के साथ
किए गए उसके अपने मापों से हैं, इसलिए नए builds में कुछ प्रतिशत का अंतर हो सकता है।

| प्रोग्राम | आकार |
|---|---|
| पाठ 1 (ऊपर वाला counter), Rust, macOS | ~0.7 MB |
| वही C / C++ में `libuison.a` के साथ, macOS | 672 KB / 816 KB |
| Studio — मुख्य demo (2,000 messages वाला mail, kanban, contacts, charts, forms), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, Visual C++ Redistributable की ज़रूरत नहीं |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, ब्राउज़र (`.wasm`, gzip) | ~220 KB |

| माप | परिणाम |
|---|---|
| event → window पर drawing पूरी, macOS | ~0.8 ms, जिसमें uison का अपना काम ~0.2 ms |
| touch → frame, iPhone 12 mini | 3.6 ms; list की scrolling एकसमान 60 fps पर |
| Studio का पूरा redraw, iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| mouse event → frame, Windows (Iris Xe, 60 Hz) | p50 1.4–1.7 ms |
| event → frame, Chrome | median 2.2 ms |
| launch → पहला frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, ब्राउज़र 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), एक egui प्रोग्राम का port, सीधी तुलना देता है: macOS
arm64 पर binary 5.1 MB के बजाय 857 KB है, `Cargo.lock` के 362 crates के बजाय कोई dependency नहीं, और
scanning की गति व memory का इस्तेमाल उतना ही।

## दस्तावेज़ीकरण

Documentation अंग्रेज़ी में है, और हर पेज का एक [रूसी संस्करण](../ru/README.md) भी है।

- [भाषा चुनें](../README.md) — कहाँ से शुरू करें।
- पाठों की श्रृंखलाएँ, हर एक अपने आप में पढ़ी जा सकती है, सिस्टम तैयार करने से लेकर threads और अपने
  frame तक: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md)।
  core पाठ तीनों भाषाओं में एक ही प्रोग्राम हैं, और उनके screenshots pixel-दर-pixel मेल खाते हैं।
- [स्क्रीन की भाषा](../screens/README.md): [bindings और values](../screens/bindings.md),
  [layout](../screens/layout.md), [elements](../screens/elements.md),
  [themes](../screens/themes.md)।
- [iOS और Android](../mobile.md), [ब्राउज़र (WebAssembly)](../wasm.md)।

## उदाहरण

| उदाहरण | क्या दिखाता है |
|---|---|
| `studio` | मुख्य demo: 2,000 messages वाला mail, kanban, contacts, animated charts, एक form, settings; 700 pt से कम पर फ़ोन वाला layout |
| `gallery` | library का हर element |
| `disk-usage` | एक egui app का port: `Canvas` node में बना sunburst chart, uison स्क्रीन के रूप में panels और menus, `std` पर parallel scanner |
| `browser` | एक छोटा web browser: अपना HTML और CSS parser और cascade, HTTP, और सिस्टम के TLS के ज़रिए HTTPS; पेज एक uison स्क्रीन बन जाता है |
| `todo` | Rust DSL में to-do list |
| `list10k` | 10,000 rows की virtualized list |
| `form2`, `settings` | forms और settings स्क्रीनें |
| `demo`, `scroll`, `uibench` | हाथ से primitives, scrolling, एक benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C और C++ के उदाहरण [`examples-c`](../../examples-c) और [`examples-cpp`](../../examples-cpp) में हैं,
हर एक अपना अलग CMake प्रोजेक्ट है।

## बिल्ड और टेस्ट

आपको stable Rust चाहिए; macOS, Windows (MSVC toolchain के साथ) और Linux पर और कुछ नहीं (`-dev`
packages भी नहीं: system libraries run time पर load होती हैं)। हर सिस्टम को तैयार करना हर श्रृंखला का
पाठ 0 है ([Rust](../rust/lessons/00-setup.md))।

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windows पर scripts `tools\*.ps1` हैं: `tools\win.ps1 studio` build करता है, एक scenario चलाता है और
screenshots save करता है, `tools\c-dist.ps1 -Test` C/C++ package को build और test करता है।

Debugging के लिए backend switches, हर जगह एक जैसे: `UISON_STATS=1` events, frames और animation का
समय print करता है, `UISON_FLASH=1` दोबारा बने regions को highlight करता है, और `UISON_SCRIPT` /
`UISON_CAPTURE` एक scenario चलाकर screenshots save करते हैं।

## संरचना

```
crates/
  uison-json      JSON pull parser और value tree, comments, trailing commas और बिना quotes वाली keys के साथ
  uison-core      geometry, रंग, fonts, events, Canvas और Host traits, App trait
  uison-frame     primitives का frame: parsing, diff → damage, hit testing, scenario runner
  uison-ui        स्क्रीन compiler: templates, bindings, themes, layout, runtime, Rust DSL, lib/*.json
  uison-apple     macOS और iOS का साझा हिस्सा: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit backend
  uison-ios       UIKit backend
  uison-android   JNI backend और उसकी Java layer (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite backend
  uison-linux     Wayland और X11 backend, Cairo + Pango, सब dlopen के ज़रिए
  uison-web       WebAssembly backend (crates/uison-web/uison.js)
  uison-c         C API और C++ headers, static और dynamic library, CMake package
  uison-gen       स्क्रीन schema से C++ widgets, Rust DSL और elements reference generate करता है
  uison-derive    syn के बिना #[derive(Data)]
  uison           umbrella crate: मौजूदा प्लेटफ़ॉर्म के लिए सब कुछ, uison::prelude; Rust के पाठ
examples/         Rust के उदाहरण (ऊपर देखें)
examples-c/       C के उदाहरण और पाठ, एक CMake प्रोजेक्ट
examples-cpp/     C++ के उदाहरण और पाठ, एक CMake प्रोजेक्ट
scenarios/        सभी प्लेटफ़ॉर्म के साझा test scenarios
tools/            हर प्लेटफ़ॉर्म के लिए build, run और test scripts; असली mouse और keyboard input के drivers
web/              WebAssembly backend का पेज और JavaScript वाला हिस्सा
docs/             अंग्रेज़ी में documentation; ru/ — वही रूसी में; readme/ — यह README दूसरी भाषाओं में
```

## सीमाएँ

- हर प्रोग्राम में एक window; drag-and-drop नहीं।
- Text input लैटिन और सिरिलिक लिपि और फ़ोन के on-screen keyboards को कवर करता है; desktop पर चीनी,
  जापानी और कोरियाई के लिए composed input (IME) अभी नहीं बना है, न ही right-to-left text।
- अभी accessibility (screen readers) का support नहीं है।
- ब्राउज़र में threads नहीं हैं।
- फ़्रेमवर्क नया है: version 0.1, API बदल सकती है।

## अनुवाद

Documentation अंग्रेज़ी में है, और `docs/ru/` में वही पेज रूसी में हैं, हर पेज के ऊपर भाषा बदलने का
switch है। यह README भी अनुवादित है: `docs/readme/README.<code>.md`, हर भाषा के लिए एक, और हर एक के
ऊपर भाषाओं की सूची। यहाँ किया गया बदलाव अनुवादों में भी किया जाना चाहिए।

## लाइसेंस

लाइसेंस अभी चुना नहीं गया है।
