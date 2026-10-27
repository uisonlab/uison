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
<b>🇵🇰 اردو</b> ·
<a href="README.id.md">🇮🇩 Bahasa Indonesia</a> ·
<a href="README.de.md">🇩🇪 Deutsch</a> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) Rust میں لکھا گیا ایک چھوٹا اور تیز UI فریم ورک ہے جس کی کوئی بیرونی ڈیپنڈنسی نہیں —
صرف اسٹینڈرڈ لائبریری، یہاں تک کہ JSON پارسر بھی اپنا ہے۔ آپ اسکرین کو ایلیمنٹس کے ایک ٹری (tree) کی صورت میں
بیان کرتے ہیں: JSON فائل میں، Rust DSL کے ذریعے، Flutter کے انداز کے C++ وِجٹس سے، یا سیدھے C کوڈ کے اندر
JSON کی شکل میں۔ لاجک Rust، C یا C++ کا عام کوڈ ہوتا ہے: ہینڈلرز، ڈیٹا، تھریڈز۔ ونڈو، لے آؤٹ، ڈرائنگ اور
ان پٹ uison سنبھالتا ہے، اور ایک ہی پروگرام macOS، Windows، Linux (Wayland اور X11)، iOS، Android اور
براؤزر میں چلتا ہے۔

ہر بیک اینڈ ہاتھ سے لکھے گئے FFI کے ذریعے سسٹم سے براہِ راست بات کرتا ہے: AppKit اور CoreText، Direct2D اور
DirectWrite کے ساتھ Win32 (COM ہاتھ سے)، شروع سے لکھا گیا Wayland پروٹوکول اور X11، UIKit، JNI کے ذریعے
Android کا Canvas، اور WebAssembly سے 2D canvas۔ ریلیز پروگرام تقریباً 0.7–0.9 MB کی ایک فائل ہے جسے سسٹم کی
اپنی لائبریریوں کے سوا کچھ نہیں چاہیے، اور ڈیسک ٹاپ پر کوئی ایونٹ تقریباً ایک ملی سیکنڈ میں اسکرین تک پہنچ جاتا ہے۔

![uison سے بنے پروگرام: Studio (میل، سیٹنگز، چارٹس)، عناصر کی گیلری، Todo، Disk Usage اور ایک چھوٹا ویب براؤزر](../img/showcase.jpg)

## یہ کیسے کام کرتا ہے

```text
اسکرین (JSON، Rust DSL، C++ وِجٹس، C میں UISON_JSON)  +  ڈیٹا  +  آپ کی زبان میں ہینڈلرز
        │  بائنڈنگز، ٹیمپلیٹس، تھیمز، لے آؤٹ (uison-ui)
        ▼
پرِمِٹِوز کا فریم: rect, line, circle, tri, poly, text, image, clip, hit
        │  پچھلے فریم سے موازنہ (diff) ← صرف بدلا ہوا حصہ دوبارہ ڈرا
        ▼
سسٹم بیک اینڈ: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

اسکرین کمپائل ہو کر ایک *فریم* بنتی ہے: چند قسم کے پرِمِٹِوز (بنیادی اشکال) کی ایک سیدھی فہرست — مستطیل، لکیریں،
دائرے، کثیر الاضلاع، متن کی اکیلی سطریں، تصاویر، کلپ ریجنز اور ماؤس اور ٹچ کے لیے نظر نہ آنے والے ہٹ ایریاز۔ ہر
نئے فریم کا پچھلے فریم سے موازنہ کیا جاتا ہے، اور بیک اینڈ صرف وہی حصے دوبارہ پینٹ کرتا ہے جو بدلے ہوں۔ بیک اینڈ
چھوٹا ہوتا ہے کیونکہ اسے صرف یہ پرِمِٹِوز ڈرا کرنے، متن ناپنے اور سسٹم کے ایونٹس کو uison کے ایونٹس میں بدلنے کا
کام کرنا ہوتا ہے۔

اسکرین کی زبان میں کوئی کوڈ نہیں ہوتا۔ متن اور پراپرٹیز ڈیٹا سے بائنڈ ہوتی ہیں (`"Pressed {@n} times"`،
`"disabled": "!@form.valid"`)، ایلیمنٹس پیرامیٹرز والے دوبارہ قابلِ استعمال ٹیمپلیٹس ہیں، رنگ اور سائز تھیم
ٹوکنز سے آتے ہیں، اور ایونٹس نام والے ایکشنز کو کال کرتے ہیں جن کے ہینڈلرز Rust، C یا C++ میں لکھے جاتے ہیں۔
جب کوئی ہینڈلر ڈیٹا بدلتا ہے تو اسکرین خود کو دوبارہ بنا لیتی ہے۔

## ہیلو، ورلڈ

تینوں زبانوں میں ایک ہی ونڈو — ایک کاؤنٹر اور "⁦+1⁩" بٹن۔

**Rust** — اسکرین DSL سے بنتی ہے، ہینڈلر بٹن پر ایک کلوژر (closure) ہے:

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

**C** — اسکرین سیدھے کوڈ میں لکھا گیا JSON ہے، ہینڈلر ایک فنکشن ہے:

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

**C++** — Flutter کی طرح وِجٹس، ہینڈلر بٹن پر ایک لیمبڈا (lambda) ہے:

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

اپنا پروجیکٹ شروع کرنے کے لیے [Rust](../rust/quickstart.md)، [C](../c/quickstart.md) یا
[C++](../cpp/quickstart.md) کی کوئیک اسٹارٹ گائیڈ دیکھیں۔

## خصوصیات

- **کوئی ڈیپنڈنسی نہیں۔** کور، JSON پارسر، لے آؤٹ، Wayland کلائنٹ، Linux سیٹنگز پورٹل کے لیے D-Bus کلائنٹ اور
  Windows کے لیے COM بائنڈنگز — سب یہیں، `std` کے اوپر لکھے گئے ہیں۔ `Cargo.lock` میں صرف ورک اسپیس کے
  اپنے crates درج ہیں۔
- **ایک پروگرام، چھ پلیٹ فارمز۔** macOS، Windows (x64 اور ARM64)، Linux (Wayland اور X11، arm64 اور x86_64)،
  iOS، Android (arm64-v8a اور armeabi-v7a) اور براؤزر (WebAssembly)۔ فون پر اسی کوڈ کو ٹچ، سیف ایریا،
  آن اسکرین کی بورڈ اور انرشیل اسکرولنگ مل جاتی ہے۔
- **جزوی ری ڈرا۔** فریمز کا diff لیا جاتا ہے اور صرف متاثرہ حصے دوبارہ پینٹ ہوتے ہیں، اور باریک لکیریں ہر بیک اینڈ
  پر یکساں طریقے سے فزیکل پکسلز پر بٹھائی جاتی ہیں۔
- **ایلیمنٹس کی لائبریری**: بٹن، آئیکن بٹن اور 25 آئیکنز، چیک باکس، ریڈیو بٹن، سوئچ، سلائیڈر، سلیکٹ، ٹیبز،
  سلیکشن اور کلپ بورڈ والے ٹیکسٹ فیلڈز، پروگریس بار، کارڈز، لسٹ ٹائلز، بیجز، اوتار، ٹول ٹپس، کانٹیکسٹ مینیو،
  لنکس اور تصاویر۔ ایلیمنٹس خود JSON ٹیمپلیٹس ہیں (`lib/*.json`)، اور آپ کے اپنے کمپوننٹس بھی اسی طرح لکھے
  جاتے ہیں۔
- **Flutter کے انداز کا لے آؤٹ**: `Row`، `Column`، `Stack`، `Wrap`، `Align`، flex، اسکرولنگ، دسیوں ہزار
  قطاروں کے لیے ورچوئلائزڈ فہرستیں، اور `Flow` — اِن لائن متن کے ٹکڑوں کا ایک پیراگراف جس میں سطریں ان ٹکڑوں کے
  آر پار ٹوٹتی ہیں۔
- **بائنڈنگز اور اسٹیٹ**: ڈیٹا کے اندر پاتھ (`@user.name`)، متن میں انٹرپولیشن، کمپوننٹ کی لوکل اسٹیٹ
  (`@~open`)، اسٹیٹ کے لحاظ سے سلیکٹرز (hover، pressed، focus)، `switch`، اور چند فنکشنز (`eq`، `not`، `len`،
  `ratio`…)۔ اسٹرکٹس ڈیٹا میں بدلتے ہیں اور ڈیٹا واپس اسٹرکٹس میں: Rust میں `#[derive(Data)]`، C میں فیلڈ ٹیبلز،
  C++20 میں میکروز کے بغیر ریفلیکشن۔
- **تھیمز**: لائٹ، ڈارک اور "سسٹم کے مطابق"، ایسے ٹوکنز سے بنی ہوئی جنہیں آپ اووررائیڈ کر سکتے ہیں۔
- **اپنی ڈرائنگ**: `Canvas` نوڈ آپ کے فنکشن کو کال کرتا ہے جو عام ایلیمنٹس کے ساتھ ساتھ اس نوڈ کے مستطیل میں
  پرِمِٹِوز لکھتا ہے۔
- **تھریڈز**: ورکر تھریڈ نتائج واپس UI تھریڈ کو بھیجتا ہے (`post`)، براؤزر کے سوا ہر پلیٹ فارم پر۔
- **لائیو ری لوڈ** ڈیبگ بلڈز میں: `UISON_RELOAD=folder` کے ساتھ اسکرین کی JSON فائلیں محفوظ کرتے ہی دوبارہ
  پڑھی جاتی ہیں۔
- **ونڈو کے بغیر ٹیسٹنگ**: ایک سیناریو (`UISON_SCRIPT=scenario.json`) ایلیمنٹ کے `key` کے ذریعے ٹیپ کرتا ہے،
  ٹائپ کرتا ہے، اسکرول کرتا ہے اور اسکرین شاٹس لیتا ہے؛ وہی سیناریوز ہر پلیٹ فارم پر، headless Chrome میں، iOS
  سمیولیٹر میں اور فون پر چلتے ہیں۔
- **C اور C++ API**: `uison.h` (C99)، `uison_kit.h` (C کے لیے مددگار)، `uison.hpp` (C++17، C++20 میں
  اضافی سہولتیں) اور `uison_widgets.hpp` (C++20 وِجٹس جو اسی اسکیما سے بنتے ہیں جس سے Rust DSL)؛ اسٹیٹک اور
  ڈائنامک لائبریریاں، CMake پیکیج اور `pkg-config`۔ C اور C++ کو Emscripten کے ساتھ براؤزر کے لیے بھی بلڈ
  کیا جا سکتا ہے: `em++ main.cpp libuison.a -o index.html`۔

## پلیٹ فارمز

| پلیٹ فارم | بیک اینڈ | کہاں جانچا گیا |
|---|---|---|
| macOS | Objective-C رن ٹائم کے ذریعے AppKit، CoreGraphics، CoreText، ImageIO | Apple Silicon Mac — ڈیولپمنٹ کی مرکزی مشین |
| iOS | UIKit، CADisplayLink، macOS والا ہی CoreGraphics/CoreText کوڈ | سمیولیٹر اور iPhone 12 mini |
| Android | ہاتھ سے JNI، `android.graphics.Canvas`، تقریباً 300 سطروں کی Java پرت؛ APK بغیر Gradle کے بنتا ہے | Huawei P40 lite (Android 10) |
| Windows | Win32، Direct2D، DirectWrite اور WIC، COM ہاتھ سے؛ ہر مانیٹر کا اپنا DPI، ڈارک ٹائٹل بار | Windows 11 PC، ‏100/125/150% پر اور مختلف اسکیل والے دو مانیٹرز کے ساتھ |
| Linux | Wayland پروٹوکول کا اپنا کوڈ اور X11 (xcb)، Cairo + Pango، xkbcommon — سب `dlopen` کے ذریعے، اس لیے بائنری صرف libc پر منحصر ہے | headless Mutter، KWin، Sway، Weston اور X11 (arm64 اور x86_64)؛ ورچوئل مشین میں GNOME کے ساتھ Ubuntu 24.04 |
| براؤزر | wasm-bindgen کے بغیر `wasm32-unknown-unknown`، canvas 2D؛ C/C++ بذریعہ Emscripten | Chrome (اور headless Chrome) |

ابھی تک نہیں جانچا گیا: Windows پر حقیقی ٹچ اسکرین، ARM پر Windows (بلڈ ہوتا ہے مگر چلایا نہیں گیا)، حقیقی
ہارڈویئر پر Linux اور KDE، GNOME میں ماؤس اور کی بورڈ ان پٹ، Safari اور Firefox۔

## سائز اور رفتار

ریلیز بلڈز (`opt-level = "s"`، LTO، `panic = "abort"`، strip شدہ)۔ سائز پروجیکٹ کی اپنی پیمائشوں سے ہیں جو
اس کے بڑھنے کے ساتھ ساتھ کی گئیں، اس لیے تازہ ترین بلڈز چند فیصد مختلف ہو سکتے ہیں۔

| پروگرام | سائز |
|---|---|
| سبق 1 (اوپر والا کاؤنٹر)، Rust، macOS | تقریباً 0.7 MB |
| یہی C / C++ میں `libuison.a` کے ساتھ، macOS | 672 KB / 816 KB |
| Studio — مرکزی ڈیمو (2,000 پیغامات والی میل، کانبان، کانٹیکٹس، چارٹس، فارمز)، macOS | تقریباً 840 KB |
| Studio، Windows x64 / ARM64 | 672 KB / 613 KB، Visual C++ Redistributable کی ضرورت نہیں |
| Studio، Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio، Android ‏`libmain.so` arm64 | تقریباً 820 KB |
| Studio، براؤزر (`.wasm`، gzip) | تقریباً 220 KB |

| پیمائش | نتیجہ |
|---|---|
| ایونٹ ← پینٹ شدہ ونڈو، macOS | تقریباً 0.8 ms، جس میں سے تقریباً 0.2 ms خود uison کا کام ہے |
| ٹچ ← فریم، iPhone 12 mini | 3.6 ms؛ فہرست کی اسکرولنگ مستقل 60 fps پر |
| Studio کا مکمل ری پینٹ، iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| ماؤس ایونٹ ← فریم، Windows ‏(Iris Xe، 60 Hz) | p50: 1.4–1.7 ms |
| ایونٹ ← فریم، Chrome | میڈین 2.2 ms |
| لانچ ← پہلا فریم | iOS 37–41 ms، Android 58–68 ms، Windows 73–103 ms، براؤزر 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md)، جو ایک egui پروگرام کا پورٹ ہے، براہِ راست موازنہ پیش کرتا ہے:
macOS arm64 پر بائنری 5.1 MB کے بجائے 857 KB ہے، `Cargo.lock` میں 362 crates کے بجائے کوئی ڈیپنڈنسی نہیں، اور
اسکیننگ کی رفتار اور میموری کا استعمال وہی ہے۔

## دستاویزات

دستاویزات انگریزی میں ہیں، اور ہر صفحے کا [روسی ورژن](../ru/README.md) بھی موجود ہے۔

- [زبان منتخب کریں](../README.md) — کہاں سے شروع کریں۔
- اسباق کے سلسلے، ہر ایک الگ سے پڑھا جا سکتا ہے، اپنا سسٹم تیار کرنے سے لے کر تھریڈز اور اپنے فریم تک:
  [Rust](../rust/README.md)، [C](../c/README.md)، [C++](../cpp/README.md)۔ بنیادی اسباق تینوں زبانوں میں
  ایک ہی پروگرام ہیں، اور ان کے اسکرین شاٹس پکسل بہ پکسل ملتے ہیں۔
- [اسکرینز کی زبان](../screens/README.md): [بائنڈنگز اور ویلیوز](../screens/bindings.md)،
  [لے آؤٹ](../screens/layout.md)، [ایلیمنٹس](../screens/elements.md)، [تھیمز](../screens/themes.md)۔
- [iOS اور Android](../mobile.md)، [براؤزر (WebAssembly)](../wasm.md)۔

## مثالیں

| مثال | کیا دکھاتی ہے |
|---|---|
| `studio` | مرکزی ڈیمو: 2,000 پیغامات والی میل، کانبان، کانٹیکٹس، اینیمیٹڈ چارٹس، ایک فارم، سیٹنگز؛ 700 pt سے کم چوڑائی کے لیے فون لے آؤٹ |
| `gallery` | لائبریری کا ہر ایلیمنٹ |
| `disk-usage` | ایک egui ایپ کا پورٹ: `Canvas` نوڈ میں بنا سن برسٹ چارٹ، uison اسکرین کی صورت میں پینلز اور مینیو، `std` پر متوازی اسکینر |
| `browser` | ایک چھوٹا ویب براؤزر: اپنا HTML اور CSS پارسر اور cascade، HTTP، اور سسٹم کے TLS کے ذریعے HTTPS؛ صفحہ uison اسکرین بن جاتا ہے |
| `todo` | Rust DSL میں ٹو ڈو لسٹ |
| `list10k` | 10,000 قطاروں کی ورچوئلائزڈ فہرست |
| `form2`، `settings` | فارمز اور سیٹنگز کی اسکرینز |
| `demo`، `scroll`، `uibench` | ہاتھ سے پرِمِٹِوز، اسکرولنگ، بینچ مارک |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C اور C++ کی مثالیں [`examples-c`](../../examples-c) اور [`examples-cpp`](../../examples-cpp) میں ہیں،
ہر ایک اپنا الگ CMake پروجیکٹ ہے۔

## بلڈ اور ٹیسٹ

آپ کو stable Rust چاہیے؛ macOS، Windows (MSVC ٹول چین کے ساتھ) اور Linux پر اس کے سوا کچھ نہیں (کوئی `-dev`
پیکیج نہیں: سسٹم لائبریریاں رن ٹائم پر لوڈ ہوتی ہیں)۔ ہر سسٹم کی تیاری ہر سلسلے کا سبق 0 ہے
([Rust](../rust/lessons/00-setup.md))۔

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windows پر اسکرپٹس `tools\*.ps1` ہیں: `tools\win.ps1 studio` بلڈ کرتا ہے، سیناریو چلاتا ہے اور اسکرین شاٹس
محفوظ کرتا ہے، `tools\c-dist.ps1 -Test` C/C++ پیکیج بلڈ اور ٹیسٹ کرتا ہے۔

ڈیبگنگ کے لیے بیک اینڈ کے سوئچز، ہر جگہ ایک جیسے: `UISON_STATS=1` ایونٹس، فریمز اور اینیمیشن کے اوقات پرنٹ
کرتا ہے، `UISON_FLASH=1` دوبارہ پینٹ ہونے والے حصوں کو نمایاں کرتا ہے، اور `UISON_SCRIPT` / `UISON_CAPTURE`
سیناریو چلاتے اور اسکرین شاٹس محفوظ کرتے ہیں۔

## ریپوزٹری کی ساخت

```
crates/
  uison-json      JSON پُل پارسر اور ویلیو ٹری، تبصروں، آخری اضافی کوموں اور بغیر کوٹس والی کیز کے ساتھ
  uison-core      جیومیٹری، رنگ، فونٹس، ایونٹس، Canvas اور Host ٹریٹس، App ٹریٹ
  uison-frame     پرِمِٹِو فریم: پارسنگ، diff ← damage، ہٹ ٹیسٹنگ، سیناریو رنر
  uison-ui        اسکرین کمپائلر: ٹیمپلیٹس، بائنڈنگز، تھیمز، لے آؤٹ، رن ٹائم، Rust DSL، lib/*.json
  uison-apple     macOS اور iOS میں مشترک: Objective-C FFI، CoreGraphics، CoreText، ImageIO
  uison-macos     AppKit بیک اینڈ
  uison-ios       UIKit بیک اینڈ
  uison-android   JNI بیک اینڈ اور اس کی Java پرت (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite بیک اینڈ
  uison-linux     Wayland اور X11 بیک اینڈ، Cairo + Pango، سب dlopen کے ذریعے
  uison-web       WebAssembly بیک اینڈ (crates/uison-web/uison.js)
  uison-c         C API اور C++ ہیڈرز، اسٹیٹک اور ڈائنامک لائبریری، CMake پیکیج
  uison-gen       اسکرین اسکیما سے C++ وِجٹس، Rust DSL اور ایلیمنٹس کا ریفرنس بناتا ہے
  uison-derive    syn کے بغیر #[derive(Data)]
  uison           امبریلا crate: موجودہ پلیٹ فارم کے لیے سب کچھ، uison::prelude؛ Rust کے اسباق
examples/         Rust مثالیں (اوپر دیکھیں)
examples-c/       C کی مثالیں اور اسباق، CMake پروجیکٹ
examples-cpp/     C++ کی مثالیں اور اسباق، CMake پروجیکٹ
scenarios/        تمام پلیٹ فارمز میں مشترک ٹیسٹ سیناریوز
tools/            ہر پلیٹ فارم کے لیے بلڈ، رن اور ٹیسٹ اسکرپٹس؛ حقیقی ماؤس اور کی بورڈ ان پٹ کے ڈرائیورز
web/              صفحہ اور WebAssembly بیک اینڈ کا JavaScript حصہ
docs/             انگریزی میں دستاویزات؛ ru/ میں یہی روسی میں؛ readme/ میں یہ README دوسری زبانوں میں
```

## حدود

- ہر پروگرام میں ایک ونڈو؛ ڈریگ اینڈ ڈراپ نہیں۔
- ٹیکسٹ ان پٹ لاطینی اور سیریلک رسم الخط اور فونز کے آن اسکرین کی بورڈز کا احاطہ کرتا ہے؛ ڈیسک ٹاپ پر چینی،
  جاپانی اور کوریائی کے لیے مرکب ان پٹ (IME) ابھی نہیں بنا، اور نہ ہی دائیں سے بائیں لکھا جانے والا متن۔
- ابھی ایکسیسبلٹی (اسکرین ریڈرز) کی سپورٹ نہیں ہے۔
- براؤزر میں تھریڈز نہیں۔
- فریم ورک نیا ہے: ورژن 0.1، API بدل سکتا ہے۔

## تراجم

دستاویزات انگریزی میں ہیں، اور `docs/ru/` میں وہی صفحات روسی میں ہیں، ہر صفحے کے اوپر زبان بدلنے کا سوئچ ہے۔ یہ
README بھی ترجمہ شدہ ہے: `docs/readme/README.<code>.md`، ہر زبان کے لیے ایک، ہر ایک کے اوپر زبانوں کی فہرست کے
ساتھ۔ یہاں کی گئی تبدیلی تراجم میں بھی ہونی چاہیے۔

## لائسنس

لائسنس ابھی منتخب نہیں کیا گیا۔
