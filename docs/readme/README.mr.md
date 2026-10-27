# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
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
<b>🇮🇳 मराठी</b> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) हे Rust मध्ये लिहिलेले एक छोटे आणि वेगवान UI फ्रेमवर्क आहे, ज्याला कोणतीही
dependency नाही — फक्त standard library, अगदी JSON parser सुद्धा स्वतःचा. तुम्ही स्क्रीन elements च्या
tree स्वरूपात लिहिता: JSON फाइलमध्ये, Rust DSL ने, Flutter सारख्या C++ widgets ने, किंवा थेट C
कोडमध्ये JSON लिहून. लॉजिक हा Rust, C किंवा C++ मधला साधा कोड असतो: handlers, डेटा, threads. window,
layout, drawing आणि input ची जबाबदारी uison घेते, आणि तोच प्रोग्राम macOS, Windows, Linux (Wayland आणि
X11), iOS, Android आणि ब्राउझरमध्ये चालतो.

प्रत्येक backend हाताने लिहिलेल्या FFI द्वारे थेट सिस्टमशी बोलतो: AppKit आणि CoreText, Direct2D आणि
DirectWrite सह Win32 (COM सुद्धा हाताने), सुरुवातीपासून लिहिलेला Wayland protocol आणि X11, UIKit, JNI
द्वारे Android चा Canvas, आणि WebAssembly मधून 2D canvas. release प्रोग्राम म्हणजे सुमारे 0.7–0.9 MB
ची एकच फाइल, जिला सिस्टमच्या स्वतःच्या libraries शिवाय काहीही लागत नाही, आणि desktop वर एखादा event
सुमारे एका मिलिसेकंदात स्क्रीनवर पोहोचतो.

![Disk Usage, uison ने बनवलेले एक उदाहरण: फोल्डरचा sunburst chart, फाइल्सची यादी आणि एक tooltip](../img/disk-usage.jpg)

## हे कसे काम करते

```text
स्क्रीन (JSON, Rust DSL, C++ widgets, C मधील UISON_JSON)  +  डेटा  +  तुमच्या भाषेतील handlers
        │  bindings, templates, themes, layout (uison-ui)
        ▼
primitives ची frame: rect, line, circle, tri, poly, text, image, clip, hit
        │  आधीच्या frame शी diff → फक्त बदललेला भाग पुन्हा काढणे
        ▼
सिस्टम backend: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

स्क्रीन एका *frame* मध्ये compile होते: काही मोजक्या प्रकारच्या primitives ची सपाट यादी — rectangles,
lines, circles, polygons, text ची एकेक ओळ, images, clip regions आणि mouse व touch साठी अदृश्य hit
areas. प्रत्येक नवी frame आधीच्या frame शी तुलना केली जाते, आणि backend फक्त बदललेले regions पुन्हा
काढतो. backend लहान आहे कारण त्याला फक्त हे primitives काढायचे, text मोजायचा आणि सिस्टमचे events uison
च्या events मध्ये बदलायचे असतात.

स्क्रीनच्या भाषेत कोड नसतो. text आणि properties डेटाशी bind होतात (`"Pressed {@n} times"`,
`"disabled": "!@form.valid"`), elements म्हणजे parameters असलेले पुन्हा वापरता येणारे templates, रंग
आणि आकार theme tokens मधून येतात, आणि events नाव दिलेल्या actions ना बोलावतात, ज्यांचे handlers Rust,
C किंवा C++ मध्ये लिहिलेले असतात. handler ने डेटा बदलला की स्क्रीन आपोआप पुन्हा तयार होते.

## Hello, world

तीच window — एक counter आणि "+1" बटण — तिन्ही भाषांमध्ये.

**Rust** — स्क्रीन DSL ने बनते, handler म्हणजे बटणावरचे closure:

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

**C** — स्क्रीन म्हणजे थेट कोडमध्ये लिहिलेले JSON, handler म्हणजे एक function:

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

**C++** — Flutter सारखे widgets, handler म्हणजे बटणावरचा lambda:

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

स्वतःचा प्रोजेक्ट सुरू करण्यासाठी [Rust](../rust/quickstart.md), [C](../c/quickstart.md) किंवा
[C++](../cpp/quickstart.md) चा quick start पाहा.

## वैशिष्ट्ये

- **कोणतीही dependency नाही.** core, JSON parser, layout, Wayland client, Linux settings portal साठी
  D-Bus client आणि Windows साठी COM bindings — सगळे इथेच `std` वर लिहिलेले आहेत. `Cargo.lock` मध्ये
  फक्त workspace चे स्वतःचे crates आहेत.
- **एक प्रोग्राम, सहा प्लॅटफॉर्म.** macOS, Windows (x64 आणि ARM64), Linux (Wayland आणि X11, arm64
  आणि x86_64), iOS, Android (arm64-v8a आणि armeabi-v7a) आणि ब्राउझर (WebAssembly). फोनवर त्याच कोडला
  touch, safe area, on-screen keyboard आणि inertial scrolling मिळते.
- **Partial redraw.** frames चा diff काढला जातो आणि फक्त बाधित regions पुन्हा काढले जातात; पातळ lines
  प्रत्येक backend वर एकाच पद्धतीने physical pixels ला align होतात.
- **Elements ची library**: buttons, icon buttons आणि 25 icons, checkboxes, radio buttons, switches,
  sliders, selects, tabs, selection आणि clipboard असलेले text fields, progress bars, cards, list
  tiles, badges, avatars, tooltips, context menus, links आणि images. elements स्वतःच JSON templates
  (`lib/*.json`) आहेत, आणि तुमचे स्वतःचे components सुद्धा याच पद्धतीने लिहिले जातात.
- **Flutter सारखे layout**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, scrolling, हजारो-हजार
  rows साठी virtualized lists, आणि `Flow` — inline text runs चा paragraph, ज्यात wrapping त्यांच्या
  पलीकडे होते.
- **Bindings आणि state**: डेटामधले paths (`@user.name`), text interpolation, component चे local
  state (`@~open`), state नुसार selectors (hover, pressed, focus), `switch`, आणि काही functions
  (`eq`, `not`, `len`, `ratio`…). structs डेटामध्ये आणि परत map होतात: Rust मध्ये `#[derive(Data)]`,
  C मध्ये field tables, C++20 मध्ये macros शिवाय reflection.
- **Themes**: light, dark आणि "सिस्टमनुसार", तुम्ही override करू शकता अशा tokens पासून बनलेल्या.
- **स्वतःचे drawing**: `Canvas` node तुमचे function बोलावतो, जे त्याच्या rectangle मध्ये primitives
  लिहिते, साध्या elements च्या शेजारी.
- **Threads**: worker thread निकाल पुन्हा UI thread कडे पाठवतो (`post`), ब्राउझर सोडून प्रत्येक
  प्लॅटफॉर्मवर.
- **Live reload** debug builds मध्ये: `UISON_RELOAD=folder` सह स्क्रीनच्या JSON फाइल्स save करताच
  पुन्हा वाचल्या जातात.
- **window शिवाय testing**: एक scenario (`UISON_SCRIPT=scenario.json`) element च्या `key` नुसार tap,
  typing, scrolling आणि screenshots करतो; तेच scenarios प्रत्येक प्लॅटफॉर्मवर, headless Chrome मध्ये,
  iOS simulator मध्ये आणि फोनवर चालतात.
- **C आणि C++ API**: `uison.h` (C99), `uison_kit.h` (C साठी helpers), `uison.hpp` (C++17, C++20 मध्ये
  अधिक सोयी) आणि `uison_widgets.hpp` (Rust DSL च्याच schema वरून generate होणारे C++20 widgets);
  static आणि dynamic libraries, एक CMake package आणि `pkg-config`. C आणि C++ Emscripten ने
  ब्राउझरसाठीही build होतात: `em++ main.cpp libuison.a -o index.html`.

## प्लॅटफॉर्म

| प्लॅटफॉर्म | backend | कुठे तपासले |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO, Objective-C runtime द्वारे | Apple Silicon Mac — मुख्य development मशीन |
| iOS | UIKit, CADisplayLink, macOS सारखाच CoreGraphics/CoreText कोड | simulator आणि iPhone 12 mini |
| Android | हाताने लिहिलेले JNI, `android.graphics.Canvas`, ~300 ओळींची Java layer; APK Gradle शिवाय build | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite आणि WIC, COM हाताने; per-monitor DPI, dark title bar | Windows 11 PC, 100/125/150 % वर, आणि वेगवेगळ्या scale चे दोन monitors |
| Linux | Wayland protocol चा स्वतःचा कोड आणि X11 (xcb), Cairo + Pango, xkbcommon — सगळे `dlopen` द्वारे, त्यामुळे binary फक्त libc वर अवलंबून | headless Mutter, KWin, Sway, Weston आणि X11 (arm64 आणि x86_64); virtual machine मध्ये GNOME सह Ubuntu 24.04 |
| ब्राउझर | wasm-bindgen शिवाय `wasm32-unknown-unknown`, canvas 2D; C/C++ Emscripten द्वारे | Chrome (आणि headless Chrome) |

अजून तपासलेले नाही: Windows वर खरी touchscreen, Windows on ARM (build होते पण अजून चालवलेले नाही), खऱ्या
hardware वर Linux आणि KDE, GNOME मध्ये mouse आणि keyboard input, Safari आणि Firefox.

## आकार आणि वेग

Release builds (`opt-level = "s"`, LTO, `panic = "abort"`, stripped). आकार प्रोजेक्ट वाढत असताना
घेतलेल्या त्याच्या स्वतःच्या मोजमापांवरून आहेत, त्यामुळे नवीन builds मध्ये काही टक्क्यांचा फरक असू शकतो.

| प्रोग्राम | आकार |
|---|---|
| धडा 1 (वरचा counter), Rust, macOS | ~0.7 MB |
| तोच C / C++ मध्ये `libuison.a` सह, macOS | 672 KB / 816 KB |
| Studio — मुख्य demo (2,000 messages असलेला mail, kanban, contacts, charts, forms), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, Visual C++ Redistributable ची गरज नाही |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, ब्राउझर (`.wasm`, gzip) | ~220 KB |

| मोजमाप | निकाल |
|---|---|
| event → window वर drawing पूर्ण, macOS | ~0.8 ms, त्यापैकी uison चे स्वतःचे काम ~0.2 ms |
| touch → frame, iPhone 12 mini | 3.6 ms; list चे scrolling एकसमान 60 fps वर |
| Studio चे पूर्ण redraw, iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| mouse event → frame, Windows (Iris Xe, 60 Hz) | p50 1.4–1.7 ms |
| event → frame, Chrome | median 2.2 ms |
| launch → पहिली frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, ब्राउझर 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), एका egui प्रोग्रामचा port, थेट तुलना देतो: macOS
arm64 वर binary 5.1 MB ऐवजी 857 KB आहे, `Cargo.lock` मधील 362 crates ऐवजी कोणतीही dependency नाही, आणि
scanning चा वेग व memory चा वापर तितकाच.

## दस्तऐवजीकरण

Documentation इंग्रजीत आहे, आणि प्रत्येक पानाची [रशियन आवृत्ती](../ru/README.md) सुद्धा आहे.

- [भाषा निवडा](../README.md) — कुठून सुरुवात करावी.
- धड्यांच्या मालिका, प्रत्येक स्वतंत्रपणे वाचता येते, सिस्टम तयार करण्यापासून threads आणि स्वतःच्या
  frame पर्यंत: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  core धडे तिन्ही भाषांमध्ये एकच प्रोग्राम आहेत, आणि त्यांचे screenshots pixel-न-pixel जुळतात.
- [स्क्रीनची भाषा](../screens/README.md): [bindings आणि values](../screens/bindings.md),
  [layout](../screens/layout.md), [elements](../screens/elements.md),
  [themes](../screens/themes.md).
- [iOS आणि Android](../mobile.md), [ब्राउझर (WebAssembly)](../wasm.md).

## उदाहरणे

| उदाहरण | काय दाखवते |
|---|---|
| `studio` | मुख्य demo: 2,000 messages असलेला mail, kanban, contacts, animated charts, एक form, settings; 700 pt पेक्षा कमी रुंदीवर फोनचे layout |
| `gallery` | library मधला प्रत्येक element |
| `disk-usage` | एका egui app चा port: `Canvas` node मध्ये काढलेला sunburst chart, uison स्क्रीन म्हणून panels आणि menus, `std` वर parallel scanner |
| `browser` | एक छोटा web browser: स्वतःचा HTML आणि CSS parser आणि cascade, HTTP, आणि सिस्टमच्या TLS द्वारे HTTPS; पान uison स्क्रीन बनते |
| `todo` | Rust DSL मधली to-do list |
| `list10k` | 10,000 rows ची virtualized list |
| `form2`, `settings` | forms आणि settings स्क्रीन्स |
| `demo`, `scroll`, `uibench` | हाताने primitives, scrolling, एक benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C आणि C++ ची उदाहरणे [`examples-c`](../../examples-c) आणि [`examples-cpp`](../../examples-cpp) मध्ये
आहेत, प्रत्येक स्वतंत्र CMake प्रोजेक्ट आहे.

## बिल्ड आणि टेस्ट

तुम्हाला stable Rust लागते; macOS, Windows (MSVC toolchain सह) आणि Linux वर आणखी काहीही नाही (`-dev`
packages सुद्धा नाहीत: system libraries run time ला load होतात). प्रत्येक सिस्टम तयार करणे हा प्रत्येक
मालिकेचा धडा 0 आहे ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windows वर scripts `tools\*.ps1` आहेत: `tools\win.ps1 studio` build करतो, एक scenario चालवतो आणि
screenshots save करतो, `tools\c-dist.ps1 -Test` C/C++ package build आणि test करतो.

Debugging साठी backend switches, सगळीकडे सारखे: `UISON_STATS=1` events, frames आणि animation चा वेळ
print करतो, `UISON_FLASH=1` पुन्हा काढलेले regions highlight करतो, आणि `UISON_SCRIPT` /
`UISON_CAPTURE` एक scenario चालवून screenshots save करतात.

## रचना

```
crates/
  uison-json      JSON pull parser आणि value tree, comments, trailing commas आणि quotes नसलेल्या keys सह
  uison-core      geometry, रंग, fonts, events, Canvas आणि Host traits, App trait
  uison-frame     primitives ची frame: parsing, diff → damage, hit testing, scenario runner
  uison-ui        स्क्रीन compiler: templates, bindings, themes, layout, runtime, Rust DSL, lib/*.json
  uison-apple     macOS आणि iOS चा सामायिक भाग: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit backend
  uison-ios       UIKit backend
  uison-android   JNI backend आणि त्याची Java layer (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite backend
  uison-linux     Wayland आणि X11 backend, Cairo + Pango, सगळे dlopen द्वारे
  uison-web       WebAssembly backend (web/uison.js)
  uison-c         C API आणि C++ headers, static आणि dynamic library, CMake package
  uison-gen       स्क्रीन schema वरून C++ widgets, Rust DSL आणि elements reference generate करतो
  uison-derive    syn शिवाय #[derive(Data)]
  uison           umbrella crate: सध्याच्या प्लॅटफॉर्मसाठी सगळे, uison::prelude; Rust चे धडे
examples/         Rust ची उदाहरणे (वर पाहा)
examples-c/       C ची उदाहरणे आणि धडे, एक CMake प्रोजेक्ट
examples-cpp/     C++ ची उदाहरणे आणि धडे, एक CMake प्रोजेक्ट
scenarios/        सर्व प्लॅटफॉर्मसाठी सामायिक test scenarios
tools/            प्रत्येक प्लॅटफॉर्मसाठी build, run आणि test scripts; खऱ्या mouse आणि keyboard input चे drivers
web/              WebAssembly backend चे पान आणि JavaScript बाजू
docs/             इंग्रजीतील documentation; ru/ — तेच रशियनमध्ये; readme/ — हे README इतर भाषांमध्ये
```

## मर्यादा

- प्रत्येक प्रोग्राममध्ये एकच window; drag-and-drop नाही.
- Text input लॅटिन आणि सिरिलिक लिपी आणि फोनचे on-screen keyboards कव्हर करते; desktop वर चिनी,
  जपानी आणि कोरियनसाठी composed input (IME) अजून केलेले नाही, right-to-left text सुद्धा नाही.
- अजून accessibility (screen readers) चा support नाही.
- ब्राउझरमध्ये threads नाहीत.
- फ्रेमवर्क नवीन आहे: version 0.1, API बदलू शकते.

## भाषांतरे

Documentation इंग्रजीत आहे, आणि `docs/ru/` मध्ये तीच पाने रशियनमध्ये आहेत, प्रत्येक पानाच्या वर भाषा
बदलण्याचा switch आहे. हे README सुद्धा भाषांतरित आहे: `docs/readme/README.<code>.md`, प्रत्येक भाषेसाठी
एक, आणि प्रत्येकाच्या वर भाषांची यादी. इथे केलेला बदल भाषांतरांमध्येही करायला हवा.

## परवाना

परवाना अजून निवडलेला नाही.
