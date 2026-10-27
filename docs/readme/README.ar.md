# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
<a href="README.es.md">🇪🇸 Español</a> ·
<a href="README.fr.md">🇫🇷 Français</a> ·
<b>🇸🇦 العربية</b> ·
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

**uison** (UI + JSON) إطار عمل صغير وسريع لواجهات المستخدم، مكتوب بلغة Rust ولا يعتمد على أي مكتبة
خارجية إطلاقًا — المكتبة القياسية فقط، وصولًا إلى محلّل JSON خاص به. تصف الشاشة على هيئة شجرة من
العناصر: في ملف JSON، أو بلغة Rust DSL، أو بعناصر واجهة (widgets) في C++ على طريقة Flutter، أو بكتابة
JSON مباشرة داخل شيفرة C. أما المنطق فشيفرة عادية بلغة Rust أو C أو C++: معالجات الأحداث والبيانات
والخيوط (threads). يتولى uison النافذة والتخطيط والرسم والإدخال، ويعمل البرنامج نفسه على macOS وWindows
وLinux (Wayland وX11) وiOS وAndroid وفي المتصفح.

تتواصل كل واجهة خلفية (backend) مع النظام مباشرة عبر FFI مكتوب يدويًا: AppKit وCoreText، وWin32 مع
Direct2D وDirectWrite (وCOM يدويًا)، وبروتوكول Wayland منفَّذ من الصفر وX11، وUIKit، وCanvas في Android
عبر JNI، ولوحة رسم 2D (canvas 2D) من WebAssembly. برنامج الإصدار (release) ملف واحد بحجم 0.7–0.9
ميغابايت تقريبًا لا يحتاج إلا إلى مكتبات النظام نفسه، ويصل الحدث إلى الشاشة في نحو ملّي ثانية واحدة على
الحاسوب المكتبي.

![برامج مبنية باستخدام uison: Studio (البريد والإعدادات والمخططات)، ومعرض العناصر، وTodo، وDisk Usage، ومتصفح ويب صغير](../img/showcase.jpg)

## كيف يعمل

```text
الشاشة (JSON، Rust DSL، عناصر C++، UISON_JSON في C)  +  البيانات  +  المعالجات بلغتك
        │  الربط، القوالب، السمات، التخطيط (uison-ui)
        ▼
إطار من العناصر الأولية: rect, line, circle, tri, poly, text, image, clip, hit
        │  المقارنة (diff) بالإطار السابق ← إعادة رسم ما تغيّر فقط
        ▼
الواجهة الخلفية للنظام: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

تُصرَّف الشاشة إلى *إطار* (frame): قائمة مسطحة من أنواع قليلة من العناصر الأولية — مستطيلات وخطوط
ودوائر ومضلعات وأسطر نص مفردة وصور ومناطق قص (clip) ومناطق نقر غير مرئية للفأرة واللمس. يُقارَن كل
إطار جديد بالإطار السابق، ولا تعيد الواجهة الخلفية رسم إلا المناطق التي تغيّرت. والواجهة الخلفية صغيرة لأن
كل ما عليها هو رسم هذه العناصر الأولية وقياس النص وتحويل أحداث النظام إلى أحداث uison.

لغة الشاشات لا تحتوي على أي شيفرة. النص والخصائص مربوطة بالبيانات (`"Pressed {@n} times"`،
`"disabled": "!@form.valid"`)، والعناصر قوالب قابلة لإعادة الاستخدام لها معاملات، والألوان والأحجام تأتي من
رموز السمة (theme tokens)، والأحداث تستدعي إجراءات مسمّاة تُكتب معالجاتها بلغة Rust أو C أو C++. وعندما
يغيّر معالجٌ البيانات، تعيد الشاشة بناء نفسها.

## مرحبًا بالعالم

النافذة نفسها — عدّاد وزر «⁦+1⁩» — باللغات الثلاث.

**Rust** — تُبنى الشاشة باستخدام DSL، والمعالج دالة مغلقة (closure) على الزر:

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

**C** — الشاشة JSON مكتوب مباشرة في الشيفرة، والمعالج دالة:

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

**C++** — عناصر واجهة كما في Flutter، والمعالج تعبير lambda على الزر:

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

للبدء بمشروعك الخاص، راجع دليل البدء السريع لـ
[Rust](../rust/quickstart.md) أو [C](../c/quickstart.md) أو [C++](../cpp/quickstart.md).

## الميزات

- **بلا اعتماديات.** النواة ومحلّل JSON والتخطيط وعميل Wayland وعميل D-Bus لبوابة إعدادات Linux
  (settings portal) وروابط COM لنظام Windows — كلها مكتوبة هنا فوق `std`. ولا يذكر `Cargo.lock` إلا
  حزم (crates) مساحة العمل نفسها.
- **برنامج واحد، ست منصات.** macOS وWindows (x64 وARM64) وLinux (Wayland وX11، arm64 وx86_64) وiOS
  وAndroid (arm64-v8a وarmeabi-v7a) والمتصفح (WebAssembly). على الهاتف تحصل الشيفرة نفسها على اللمس
  والمنطقة الآمنة (safe area) ولوحة المفاتيح على الشاشة والتمرير بالقصور الذاتي.
- **إعادة رسم جزئية.** تُقارَن الإطارات ولا يُعاد رسم إلا المناطق المتضررة، مع محاذاة الخطوط الرفيعة إلى
  البكسلات الفعلية بالطريقة نفسها في كل الواجهات الخلفية.
- **مكتبة عناصر**: أزرار وأزرار أيقونات و25 أيقونة، ومربعات اختيار، وأزرار اختيار (radio)، ومفاتيح تبديل،
  ومنزلقات، وقوائم منسدلة، وتبويبات، وحقول نص مع التحديد والحافظة، وأشرطة تقدّم، وبطاقات، وعناصر قوائم
  (list tiles)، وشارات، وصور رمزية (avatars)، وتلميحات، وقوائم سياقية، وروابط، وصور. العناصر نفسها قوالب
  JSON ‏(`lib/*.json`)، ومكوّناتك الخاصة تُكتب بالطريقة ذاتها.
- **تخطيط على طريقة Flutter**: `Row` و`Column` و`Stack` و`Wrap` و`Align` وflex والتمرير، وقوائم افتراضية
  (virtualized) لعشرات آلاف الصفوف، و`Flow` — فقرة من مقاطع نص متتالية مع التفاف الأسطر عبرها.
- **الربط والحالة**: مسارات داخل البيانات (`@user.name`)، واستيفاء النص، والحالة المحلية للمكوّن (`@~open`)،
  ومحدِّدات حسب الحالة (hover وpressed وfocus)، و`switch`، وبعض الدوال (`eq`، `not`، `len`، `ratio`…).
  تتحوّل البنى (structs) إلى بيانات والعكس: `#[derive(Data)]` في Rust، وجداول الحقول في C، والانعكاس
  (reflection) بلا ماكروهات في C++20.
- **السمات (themes)**: فاتحة وداكنة و«تتبع النظام»، مبنية من رموز (tokens) يمكنك تجاوزها.
- **رسمك الخاص**: تستدعي العقدة `Canvas` دالتك لتكتب العناصر الأولية في مستطيلها، إلى جانب العناصر العادية.
- **الخيوط**: يرسل خيط عامل (worker thread) النتائج إلى خيط الواجهة (`post`)، على كل المنصات ما عدا
  المتصفح.
- **إعادة التحميل الحي** في نسخ التصحيح (debug): مع `UISON_RELOAD=folder` تُعاد قراءة ملفات JSON الخاصة
  بالشاشة عند حفظها.
- **الاختبار بلا نافذة**: سيناريو (`UISON_SCRIPT=scenario.json`) ينقر ويكتب ويمرّر ويلتقط لقطات شاشة حسب
  `key` العنصر؛ والسيناريوهات نفسها تعمل على كل المنصات، وفي Chrome بلا واجهة (headless)، وفي محاكي iOS،
  وعلى الهاتف.
- **واجهة برمجية لـ C وC++**: `uison.h` (C99)، و`uison_kit.h` (أدوات مساعدة لـ C)، و`uison.hpp` (C++17،
  مع تسهيلات في C++20)، و`uison_widgets.hpp` (عناصر C++20 مولَّدة من المخطط نفسه الذي يُولَّد منه Rust DSL)؛
  مكتبات ثابتة وديناميكية، وحزمة CMake، و`pkg-config`. ويمكن أيضًا بناء C وC++ للمتصفح باستخدام
  Emscripten: `em++ main.cpp libuison.a -o index.html`.

## المنصات

| المنصة | الواجهة الخلفية | جرى التحقق عليها |
|---|---|---|
| macOS | AppKit وCoreGraphics وCoreText وImageIO عبر بيئة تشغيل Objective-C | Mac بمعالج Apple Silicon — جهاز التطوير الرئيسي |
| iOS | UIKit وCADisplayLink، وشيفرة CoreGraphics/CoreText نفسها المستخدمة في macOS | المحاكي وiPhone 12 mini |
| Android | JNI يدويًا، و`android.graphics.Canvas`، وطبقة Java من نحو 300 سطر؛ يُبنى APK دون Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32 وDirect2D وDirectWrite وWIC، وCOM يدويًا؛ DPI لكل شاشة، وشريط عنوان داكن | حاسوب بنظام Windows 11 بمقاييس 100/125/150% وشاشتان بمقياسين مختلفين |
| Linux | شيفرة خاصة لبروتوكول Wayland وX11 ‏(xcb)، وCairo + Pango، وxkbcommon — كلها عبر `dlopen`، فلا يعتمد الملف التنفيذي إلا على libc | Mutter وKWin وSway وWeston وX11 بلا شاشة (arm64 وx86_64)؛ Ubuntu 24.04 مع GNOME في آلة افتراضية |
| المتصفح | `wasm32-unknown-unknown` دون wasm-bindgen، وcanvas 2D؛ وC/C++ عبر Emscripten | Chrome (وChrome بلا واجهة) |

لم يُتحقَّق منه بعد: شاشة لمس حقيقية على Windows، وWindows على ARM (يُبنى لكنه لم يُشغَّل)، وLinux على عتاد
حقيقي وKDE، وإدخال الفأرة ولوحة المفاتيح في GNOME، وSafari وFirefox.

## الحجم والسرعة

نسخ الإصدار (`opt-level = "s"`، وLTO، و`panic = "abort"`، مع إزالة الرموز). الأحجام مأخوذة من قياسات
المشروع نفسه أثناء نموّه، لذا قد تختلف أحدث النسخ ببضعة بالمئة.

| البرنامج | الحجم |
|---|---|
| الدرس 1 (العدّاد أعلاه)، Rust، macOS | نحو 0.7 ميغابايت |
| الشيء نفسه بلغة C / C++ مع `libuison.a`، macOS | 672 كيلوبايت / 816 كيلوبايت |
| Studio — العرض التوضيحي الرئيسي (بريد بـ 2,000 رسالة، ولوحة كانبان، وجهات اتصال، ومخططات، ونماذج)، macOS | نحو 840 كيلوبايت |
| Studio، Windows x64 / ARM64 | 672 كيلوبايت / 613 كيلوبايت، دون الحاجة إلى Visual C++ Redistributable |
| Studio، Linux arm64 / x86_64 | 857 كيلوبايت / 936 كيلوبايت |
| Studio، Android ‏`libmain.so` arm64 | نحو 820 كيلوبايت |
| Studio، المتصفح (`.wasm`، gzip) | نحو 220 كيلوبايت |

| القياس | النتيجة |
|---|---|
| حدث ← نافذة مرسومة، macOS | نحو 0.8 ملّي ثانية، منها نحو 0.2 ملّي ثانية عمل uison نفسه |
| لمسة ← إطار، iPhone 12 mini | 3.6 ملّي ثانية؛ تمرير قائمة بمعدل ثابت قدره 60 إطارًا في الثانية |
| إعادة رسم Studio بالكامل، iPhone 12 mini / Huawei P40 lite | 0.7 ملّي ثانية / 0.4–0.7 ملّي ثانية |
| حدث فأرة ← إطار، Windows ‏(Iris Xe، 60 هرتز) | p50: 1.4–1.7 ملّي ثانية |
| حدث ← إطار، Chrome | الوسيط 2.2 ملّي ثانية |
| التشغيل ← أول إطار | iOS 37–41 ملّي ثانية، Android 58–68 ملّي ثانية، Windows 73–103 ملّي ثانية، المتصفح 24–42 ملّي ثانية |

يقدّم [Disk Usage](../../examples/disk-usage/README.md)، وهو نقل (port) لبرنامج مكتوب بـ egui، مقارنة مباشرة:
على macOS arm64 يبلغ حجم الملف التنفيذي 857 كيلوبايت بدلًا من 5.1 ميغابايت، دون أي اعتماديات بدلًا من 362
حزمة (crate) في `Cargo.lock`، وبسرعة الفحص نفسها واستهلاك الذاكرة نفسه.

## التوثيق

التوثيق باللغة الإنجليزية، مع [نسخة روسية](../ru/README.md) من كل صفحة.

- [اختر لغة](../README.md) — من أين تبدأ.
- سلاسل دروس، يمكن قراءة كل منها على حدة، من إعداد نظامك إلى الخيوط وإطارك الخاص:
  [Rust](../rust/README.md)، [C](../c/README.md)، [C++](../cpp/README.md). الدروس الأساسية هي البرنامج
  نفسه باللغات الثلاث، ولقطات شاشتها متطابقة بكسلًا ببكسل.
- [لغة الشاشات](../screens/README.md): [الربط والقيم](../screens/bindings.md)،
  [التخطيط](../screens/layout.md)، [العناصر](../screens/elements.md)، [السمات](../screens/themes.md).
- [iOS وAndroid](../mobile.md)، [المتصفح (WebAssembly)](../wasm.md).

## الأمثلة

| المثال | ما يعرضه |
|---|---|
| `studio` | العرض التوضيحي الرئيسي: بريد بـ 2,000 رسالة، ولوحة كانبان، وجهات اتصال، ومخططات متحركة، ونموذج، وإعدادات؛ وتخطيط للهاتف لعرض أقل من 700 pt |
| `gallery` | كل عناصر المكتبة |
| `disk-usage` | نقل لتطبيق egui: مخطط شمسي مرسوم في عقدة `Canvas`، ولوحات وقوائم على هيئة شاشة uison، وماسح متوازٍ مبني على `std` |
| `browser` | متصفح ويب صغير: محلّل HTML وCSS وتتالٍ (cascade) خاص به، وHTTP، وHTTPS عبر TLS الخاص بالنظام؛ تتحوّل الصفحة إلى شاشة uison |
| `todo` | قائمة مهام بلغة Rust DSL |
| `list10k` | قائمة افتراضية من 10,000 صف |
| `form2`، `settings` | شاشات النماذج والإعدادات |
| `demo`، `scroll`، `uibench` | عناصر أولية يدويًا، والتمرير، واختبار أداء (benchmark) |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

توجد أمثلة C وC++ في [`examples-c`](../../examples-c) و[`examples-cpp`](../../examples-cpp)،
ولكلٍّ منهما مشروع CMake خاص به.

## البناء والاختبار

تحتاج إلى Rust المستقر (stable)، ولا شيء غيره على macOS وWindows (مع سلسلة أدوات MSVC) وLinux
(دون حزم `-dev`: تُحمَّل مكتبات النظام وقت التشغيل). إعداد كل نظام هو الدرس 0 في كل سلسلة
([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

على Windows السكربتات هي `tools\*.ps1`: يبني `tools\win.ps1 studio` البرنامج ويشغّل سيناريو ويحفظ لقطات
الشاشة، ويبني `tools\c-dist.ps1 -Test` حزمة C/C++ ويختبرها.

مفاتيح الواجهة الخلفية للتصحيح، وهي نفسها في كل مكان: يطبع `UISON_STATS=1` توقيتات الأحداث والإطارات
والحركة، ويُبرز `UISON_FLASH=1` المناطق المعاد رسمها، ويشغّل `UISON_SCRIPT` / `UISON_CAPTURE` سيناريو
ويحفظان لقطات الشاشة.

## بنية المستودع

```
crates/
  uison-json      محلّل JSON بالسحب (pull parser) وشجرة قيم، مع التعليقات والفواصل الزائدة والمفاتيح بلا علامات اقتباس
  uison-core      الهندسة، الألوان، الخطوط، الأحداث، الـ traits: Canvas وHost وApp
  uison-frame     الإطار الأولي: التحليل، المقارنة (diff) ← مناطق التلف (damage)، اختبار النقر (hit testing)، منفّذ السيناريوهات
  uison-ui        مصرِّف الشاشات: القوالب، الربط، السمات، التخطيط، بيئة التشغيل، Rust DSL، lib/*.json
  uison-apple     مشترك بين macOS وiOS: Objective-C FFI، CoreGraphics، CoreText، ImageIO
  uison-macos     واجهة خلفية لـ AppKit
  uison-ios       واجهة خلفية لـ UIKit
  uison-android   واجهة خلفية عبر JNI مع طبقة Java الخاصة بها (java/dev/uison)
  uison-windows   واجهة خلفية Win32 + Direct2D + DirectWrite
  uison-linux     واجهة خلفية Wayland وX11، وCairo + Pango، وكلها عبر dlopen
  uison-web       واجهة خلفية WebAssembly (crates/uison-web/uison.js)
  uison-c         واجهة C البرمجية وترويسات C++، مكتبة ثابتة وديناميكية، حزمة CMake
  uison-gen       يولّد عناصر C++ وRust DSL ومرجع العناصر من مخطط الشاشات
  uison-derive    #[derive(Data)] دون syn
  uison           الحزمة الجامعة (umbrella crate): كل شيء للمنصة الحالية، uison::prelude؛ دروس Rust
examples/         أمثلة Rust (انظر أعلاه)
examples-c/       أمثلة ودروس C، مشروع CMake
examples-cpp/     أمثلة ودروس C++، مشروع CMake
scenarios/        سيناريوهات اختبار مشتركة بين كل المنصات
tools/            سكربتات البناء والتشغيل والاختبار لكل المنصات؛ مشغّلات إدخال حقيقي بالفأرة ولوحة المفاتيح
web/              الصفحة وجانب JavaScript من الواجهة الخلفية لـ WebAssembly
docs/             التوثيق بالإنجليزية؛ ru/ النسخة نفسها بالروسية؛ readme/ هذا الـ README بلغات أخرى
```

## القيود

- نافذة واحدة لكل برنامج؛ لا يوجد سحب وإفلات (drag-and-drop).
- يغطي إدخال النص الأبجديتين اللاتينية والسيريلية ولوحات المفاتيح على شاشات الهواتف؛ أما الإدخال المركّب
  للصينية واليابانية والكورية (IME) على الحاسوب المكتبي فلم يُنجَز بعد، ولا النص من اليمين إلى اليسار.
- لا دعم لإمكانية الوصول (قارئات الشاشة) حتى الآن.
- لا خيوط في المتصفح.
- إطار العمل حديث العهد: الإصدار 0.1، وقد تتغيّر الواجهة البرمجية (API).

## الترجمات

التوثيق باللغة الإنجليزية، ويضم `docs/ru/` الصفحات نفسها بالروسية، مع مبدّل لغة في أعلى كل صفحة. وهذا
الـ README مترجم أيضًا: `docs/readme/README.<code>.md`، ملف لكل لغة، مع قائمة اللغات في أعلى كل منها.
وأي تغيير هنا ينبغي أن يُنقل إلى الترجمات أيضًا.

## الترخيص

لم يُختَر الترخيص بعد.
