# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
<a href="README.es.md">🇪🇸 Español</a> ·
<a href="README.fr.md">🇫🇷 Français</a> ·
<a href="README.ar.md">🇸🇦 العربية</a> ·
<b>🇧🇩 বাংলা</b> ·
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

**uison** (UI + JSON) হলো Rust-এ লেখা একটি ছোট, দ্রুত UI ফ্রেমওয়ার্ক, যার কোনো বাইরের ডিপেন্ডেন্সি নেই —
শুধু স্ট্যান্ডার্ড লাইব্রেরি, এমনকি JSON পার্সারটাও নিজস্ব। আপনি একটি স্ক্রিনকে এলিমেন্টের ট্রি হিসেবে বর্ণনা
করেন: একটি JSON ফাইলে, Rust DSL দিয়ে, Flutter-এর ধাঁচে C++ উইজেট দিয়ে, অথবা সরাসরি C কোডের ভেতরে JSON
হিসেবে। লজিক হলো Rust, C বা C++-এর সাধারণ কোড: হ্যান্ডলার, ডেটা, থ্রেড। উইন্ডো, লেআউট, ড্রয়িং আর ইনপুটের
দায়িত্ব uison-এর, এবং একই প্রোগ্রাম macOS, Windows, Linux (Wayland ও X11), iOS, Android এবং ব্রাউজারে চলে।

প্রতিটি ব্যাকএন্ড হাতে লেখা FFI দিয়ে সরাসরি সিস্টেমের সঙ্গে কথা বলে: AppKit ও CoreText, Direct2D ও DirectWrite
সহ Win32 (COM হাতে লেখা), শূন্য থেকে বানানো Wayland প্রোটোকল ও X11, UIKit, JNI-র মাধ্যমে Android-এর Canvas,
এবং WebAssembly থেকে 2D canvas। একটি রিলিজ প্রোগ্রাম প্রায় 0.7–0.9 MB-এর একটিমাত্র ফাইল, সিস্টেমের নিজস্ব
লাইব্রেরি ছাড়া যার আর কিছু লাগে না, আর ডেস্কটপে একটি ইভেন্ট প্রায় এক মিলিসেকেন্ডে স্ক্রিনে পৌঁছায়।

![uison দিয়ে তৈরি প্রোগ্রাম: Studio (মেইল, সেটিংস, চার্ট), এলিমেন্ট গ্যালারি, Todo, Disk Usage এবং একটি ছোট ওয়েব ব্রাউজার](../img/showcase.jpg)

## এটি কীভাবে কাজ করে

```text
স্ক্রিন (JSON, Rust DSL, C++ উইজেট, C-তে UISON_JSON)  +  ডেটা  +  আপনার ভাষায় হ্যান্ডলার
        │  বাইন্ডিং, টেমপ্লেট, থিম, লেআউট (uison-ui)
        ▼
প্রিমিটিভের ফ্রেম: rect, line, circle, tri, poly, text, image, clip, hit
        │  আগের ফ্রেমের সঙ্গে diff → শুধু যা বদলেছে তা আবার আঁকা
        ▼
সিস্টেম ব্যাকএন্ড: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

একটি স্ক্রিন কম্পাইল হয়ে একটি *ফ্রেম*-এ পরিণত হয়: অল্প কয়েক ধরনের প্রিমিটিভের একটি সমতল তালিকা —
আয়তক্ষেত্র, রেখা, বৃত্ত, বহুভুজ, এক লাইনের টেক্সট, ছবি, ক্লিপ অঞ্চল, এবং মাউস ও টাচের জন্য অদৃশ্য হিট এরিয়া।
প্রতিটি নতুন ফ্রেমকে আগের ফ্রেমের সঙ্গে তুলনা করা হয়, আর ব্যাকএন্ড শুধু বদলে যাওয়া অঞ্চলগুলোই আবার আঁকে।
ব্যাকএন্ড ছোট, কারণ তাকে শুধু এই প্রিমিটিভগুলো আঁকতে, টেক্সট মাপতে এবং সিস্টেমের ইভেন্টকে uison-এর ইভেন্টে
রূপান্তর করতে হয়।

স্ক্রিনের ভাষায় কোনো কোড নেই। টেক্সট আর প্রপার্টি ডেটার সঙ্গে বাঁধা থাকে (`"Pressed {@n} times"`,
`"disabled": "!@form.valid"`), এলিমেন্টগুলো প্যারামিটারসহ পুনর্ব্যবহারযোগ্য টেমপ্লেট, রং ও আকার আসে থিম টোকেন
থেকে, আর ইভেন্টগুলো নামযুক্ত অ্যাকশন কল করে, যাদের হ্যান্ডলার লেখা হয় Rust, C বা C++-এ। কোনো হ্যান্ডলার ডেটা
বদলালে স্ক্রিন নিজেকে আবার গড়ে নেয়।

## হ্যালো, ওয়ার্ল্ড

একই উইন্ডো — একটি কাউন্টার আর একটি "+1" বোতাম — তিনটি ভাষায়।

**Rust** — স্ক্রিন বানানো হয় DSL দিয়ে, হ্যান্ডলার হলো বোতামের ওপর একটি ক্লোজার:

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

**C** — স্ক্রিন হলো সরাসরি কোডে লেখা JSON, হ্যান্ডলার একটি ফাংশন:

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

**C++** — Flutter-এর মতো উইজেট, হ্যান্ডলার বোতামের ওপর একটি ল্যাম্বডা:

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

নিজের প্রজেক্ট শুরু করতে [Rust](../rust/quickstart.md), [C](../c/quickstart.md) বা
[C++](../cpp/quickstart.md)-এর কুইক স্টার্ট দেখুন।

## বৈশিষ্ট্য

- **কোনো ডিপেন্ডেন্সি নেই।** কোর, JSON পার্সার, লেআউট, Wayland ক্লায়েন্ট, Linux সেটিংস পোর্টালের জন্য D-Bus
  ক্লায়েন্ট আর Windows-এর COM বাইন্ডিং — সবই এখানে, `std`-এর ওপর লেখা। `Cargo.lock`-এ শুধু ওয়ার্কস্পেসের
  নিজস্ব crate-গুলোই আছে।
- **এক প্রোগ্রাম, ছয় প্ল্যাটফর্ম।** macOS, Windows (x64 ও ARM64), Linux (Wayland ও X11, arm64 ও x86_64),
  iOS, Android (arm64-v8a ও armeabi-v7a) এবং ব্রাউজার (WebAssembly)। ফোনে একই কোড পায় টাচ, সেফ এরিয়া,
  অন-স্ক্রিন কিবোর্ড আর ইনার্শিয়াল স্ক্রলিং।
- **আংশিক রিড্র (partial redraw)।** ফ্রেমগুলোর diff নেওয়া হয় এবং শুধু ক্ষতিগ্রস্ত অঞ্চল আবার আঁকা হয়; সরু
  রেখাগুলো প্রতিটি ব্যাকএন্ডে একইভাবে ফিজিক্যাল পিক্সেলে বসানো হয়।
- **এলিমেন্ট লাইব্রেরি**: বোতাম, আইকন বোতাম ও 25টি আইকন, চেকবক্স, রেডিও বোতাম, সুইচ, স্লাইডার, সিলেক্ট,
  ট্যাব, সিলেকশন ও ক্লিপবোর্ডসহ টেক্সট ফিল্ড, প্রোগ্রেস বার, কার্ড, লিস্ট টাইল, ব্যাজ, অবতার, টুলটিপ, কনটেক্সট
  মেনু, লিংক ও ছবি। এলিমেন্টগুলো নিজেরাই JSON টেমপ্লেট (`lib/*.json`), আর আপনার নিজের কম্পোনেন্টও একইভাবে
  লেখা হয়।
- **Flutter-এর ধাঁচে লেআউট**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, স্ক্রলিং, কয়েক দশ হাজার
  সারির জন্য ভার্চুয়ালাইজড তালিকা, এবং `Flow` — ইনলাইন টেক্সট অংশের একটি অনুচ্ছেদ, যেখানে লাইন ভাঙে সেই
  অংশগুলোর সীমানা পেরিয়ে।
- **বাইন্ডিং ও স্টেট**: ডেটার ভেতরের পাথ (`@user.name`), টেক্সট ইন্টারপোলেশন, কম্পোনেন্টের লোকাল স্টেট
  (`@~open`), স্টেট অনুযায়ী সিলেক্টর (hover, pressed, focus), `switch`, এবং কয়েকটি ফাংশন (`eq`, `not`, `len`,
  `ratio`…)। স্ট্রাক্ট থেকে ডেটা আর ডেটা থেকে স্ট্রাক্ট: Rust-এ `#[derive(Data)]`, C-তে ফিল্ড টেবিল, C++20-এ
  ম্যাক্রো ছাড়া রিফ্লেকশন।
- **থিম**: লাইট, ডার্ক এবং "সিস্টেম অনুসরণ", এমন টোকেন দিয়ে তৈরি যা আপনি ওভাররাইড করতে পারেন।
- **নিজস্ব ড্রয়িং**: একটি `Canvas` নোড আপনার ফাংশন কল করে, যা সাধারণ এলিমেন্টের পাশাপাশি নোডের আয়তক্ষেত্রে
  প্রিমিটিভ লেখে।
- **থ্রেড**: একটি ওয়ার্কার থ্রেড ফলাফল UI থ্রেডে ফেরত পাঠায় (`post`), ব্রাউজার ছাড়া সব প্ল্যাটফর্মে।
- **লাইভ রিলোড** ডিবাগ বিল্ডে: `UISON_RELOAD=folder` দিলে স্ক্রিনের JSON ফাইলগুলো সেভ করার সঙ্গে সঙ্গে আবার
  পড়া হয়।
- **উইন্ডো ছাড়া টেস্টিং**: একটি সিনারিও (`UISON_SCRIPT=scenario.json`) এলিমেন্টের `key` ধরে ট্যাপ করে, টাইপ
  করে, স্ক্রল করে এবং স্ক্রিনশট নেয়; একই সিনারিও সব প্ল্যাটফর্মে, headless Chrome-এ, iOS সিমুলেটরে এবং ফোনে
  চলে।
- **[এজেন্ট মোড](../agents.md)**: একটি LLM এজেন্ট স্ক্রিনশট ছাড়াই প্রোগ্রাম ব্যবহার করে। সে স্ক্রিনকে অর্থ দিয়ে পড়ে (ভূমিকা, নাম, অবস্থা, key)
  এবং মানুষের ইনপুটের একই পথে ক্লিক, টাইপ ও নির্বাচন করে — JSON লাইনে (`--agent`) বা MCP সার্ভার হিসেবে (`--mcp`)।
- **C ও C++ API**: `uison.h` (C99), `uison_kit.h` (C-এর জন্য সহায়ক), `uison.hpp` (C++17, C++20-এ বাড়তি
  সুবিধা) এবং `uison_widgets.hpp` (C++20 উইজেট, Rust DSL যে স্কিমা থেকে তৈরি সেই একই স্কিমা থেকে জেনারেট
  করা); স্ট্যাটিক ও ডায়নামিক লাইব্রেরি, একটি CMake প্যাকেজ এবং `pkg-config`। Emscripten দিয়ে C ও C++
  ব্রাউজারের জন্যও বিল্ড করা যায়: `em++ main.cpp libuison.a -o index.html`।

## প্ল্যাটফর্ম

| প্ল্যাটফর্ম | ব্যাকএন্ড | যেখানে যাচাই করা হয়েছে |
|---|---|---|
| macOS | Objective-C রানটাইমের মাধ্যমে AppKit, CoreGraphics, CoreText, ImageIO | Apple Silicon Mac — মূল ডেভেলপমেন্ট মেশিন |
| iOS | UIKit, CADisplayLink, macOS-এর মতোই একই CoreGraphics/CoreText কোড | সিমুলেটর ও iPhone 12 mini |
| Android | হাতে লেখা JNI, `android.graphics.Canvas`, প্রায় 300 লাইনের একটি Java স্তর; Gradle ছাড়াই APK বিল্ড | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite ও WIC, হাতে লেখা COM; প্রতি-মনিটর DPI, ডার্ক টাইটেল বার | Windows 11 PC, 100/125/150 %-এ এবং ভিন্ন স্কেলের দুটি মনিটরে |
| Linux | Wayland প্রোটোকলের নিজস্ব কোড ও X11 (xcb), Cairo + Pango, xkbcommon — সবই `dlopen`-এর মাধ্যমে, তাই বাইনারি শুধু libc-এর ওপর নির্ভর করে | headless Mutter, KWin, Sway, Weston ও X11 (arm64 ও x86_64); ভার্চুয়াল মেশিনে GNOME সহ Ubuntu 24.04 |
| ব্রাউজার | wasm-bindgen ছাড়া `wasm32-unknown-unknown`, canvas 2D; Emscripten-এর মাধ্যমে C/C++ | Chrome (ও headless Chrome) |

এখনও যাচাই হয়নি: Windows-এ আসল টাচস্ক্রিন, ARM-এ Windows (বিল্ড হয়, কিন্তু চালানো হয়নি), আসল হার্ডওয়্যারে
Linux ও KDE, GNOME-এ মাউস ও কিবোর্ড ইনপুট, Safari ও Firefox।

## আকার ও গতি

রিলিজ বিল্ড (`opt-level = "s"`, LTO, `panic = "abort"`, strip করা)। আকারগুলো প্রজেক্ট বড় হওয়ার সময় তার
নিজস্ব পরিমাপ থেকে নেওয়া, তাই সবচেয়ে নতুন বিল্ড কয়েক শতাংশ আলাদা হতে পারে।

| প্রোগ্রাম | আকার |
|---|---|
| পাঠ 1 (ওপরের কাউন্টার), Rust, macOS | ~0.7 MB |
| একই প্রোগ্রাম C / C++-এ `libuison.a` দিয়ে, macOS | 672 KB / 816 KB |
| Studio — মূল ডেমো (2,000 বার্তাসহ মেইল, কানবান, কন্টাক্ট, চার্ট, ফর্ম), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, Visual C++ Redistributable লাগে না |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, ব্রাউজার (`.wasm`, gzip) | ~220 KB |

| পরিমাপ | ফলাফল |
|---|---|
| ইভেন্ট → আঁকা উইন্ডো, macOS | ~0.8 ms, এর মধ্যে ~0.2 ms uison-এর নিজের কাজ |
| টাচ → ফ্রেম, iPhone 12 mini | 3.6 ms; স্থির 60 fps-এ তালিকা স্ক্রলিং |
| Studio-র পূর্ণ রিপেইন্ট, iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| মাউস ইভেন্ট → ফ্রেম, Windows (Iris Xe, 60 Hz) | p50 1.4–1.7 ms |
| ইভেন্ট → ফ্রেম, Chrome | মিডিয়ান 2.2 ms |
| লঞ্চ → প্রথম ফ্রেম | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, ব্রাউজার 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), একটি egui প্রোগ্রামের পোর্ট, সরাসরি তুলনার সুযোগ দেয়:
macOS arm64-এ বাইনারি 5.1 MB-এর বদলে 857 KB, `Cargo.lock`-এ 362টি crate-এর বদলে কোনো ডিপেন্ডেন্সি নেই, আর
স্ক্যানিংয়ের গতি ও মেমরি ব্যবহার একই।

## ডকুমেন্টেশন

ডকুমেন্টেশন ইংরেজিতে, প্রতিটি পৃষ্ঠার একটি [রুশ সংস্করণ](../ru/README.md) সহ।

- [ভাষা বেছে নিন](../README.md) — কোথা থেকে শুরু করবেন।
- পাঠের সিরিজ, প্রতিটি আলাদাভাবে পড়া যায়, সিস্টেম প্রস্তুত করা থেকে থ্রেড ও নিজস্ব ফ্রেম পর্যন্ত:
  [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md)। মূল পাঠগুলো তিন ভাষাতেই একই
  প্রোগ্রাম, আর তাদের স্ক্রিনশট পিক্সেল-বাই-পিক্সেল মেলে।
- [স্ক্রিনের ভাষা](../screens/README.md): [বাইন্ডিং ও মান](../screens/bindings.md),
  [লেআউট](../screens/layout.md), [এলিমেন্ট](../screens/elements.md), [থিম](../screens/themes.md)।
- [iOS ও Android](../mobile.md), [ব্রাউজার (WebAssembly)](../wasm.md)।

## উদাহরণ

| উদাহরণ | কী দেখায় |
|---|---|
| `studio` | মূল ডেমো: 2,000 বার্তাসহ মেইল, কানবান, কন্টাক্ট, অ্যানিমেটেড চার্ট, একটি ফর্ম, সেটিংস; 700 pt-এর কম প্রস্থের জন্য ফোন লেআউট |
| `gallery` | লাইব্রেরির প্রতিটি এলিমেন্ট |
| `disk-usage` | একটি egui অ্যাপের পোর্ট: `Canvas` নোডে আঁকা সানবার্স্ট চার্ট, uison স্ক্রিন হিসেবে প্যানেল ও মেনু, `std`-এর ওপর সমান্তরাল স্ক্যানার |
| `browser` | একটি ছোট ওয়েব ব্রাউজার: নিজস্ব HTML ও CSS পার্সার এবং ক্যাসকেড, HTTP, আর সিস্টেমের TLS দিয়ে HTTPS; একটি পৃষ্ঠা uison স্ক্রিনে পরিণত হয় |
| `todo` | Rust DSL-এ একটি টু-ডু তালিকা |
| `list10k` | 10,000 সারির একটি ভার্চুয়ালাইজড তালিকা |
| `form2`, `settings` | ফর্ম ও সেটিংস স্ক্রিন |
| `demo`, `scroll`, `uibench` | হাতে আঁকা প্রিমিটিভ, স্ক্রলিং, একটি বেঞ্চমার্ক |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C ও C++ উদাহরণগুলো আছে [`examples-c`](../../examples-c) ও [`examples-cpp`](../../examples-cpp)-এ,
প্রতিটি নিজস্ব একটি CMake প্রজেক্ট।

## বিল্ড ও টেস্ট

আপনার লাগবে stable Rust; macOS, Windows (MSVC টুলচেইনসহ) ও Linux-এ আর কিছু নয় (কোনো `-dev` প্যাকেজ লাগে না:
সিস্টেম লাইব্রেরি রানটাইমে লোড হয়)। প্রতিটি সিস্টেম প্রস্তুত করা প্রতিটি সিরিজের পাঠ 0
([Rust](../rust/lessons/00-setup.md))।

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windows-এ স্ক্রিপ্টগুলো `tools\*.ps1`: `tools\win.ps1 studio` বিল্ড করে, একটি সিনারিও চালায় ও স্ক্রিনশট সেভ
করে, `tools\c-dist.ps1 -Test` C/C++ প্যাকেজ বিল্ড ও টেস্ট করে।

ডিবাগিংয়ের জন্য ব্যাকএন্ডের সুইচ, সব জায়গায় একই: `UISON_STATS=1` ইভেন্ট, ফ্রেম ও অ্যানিমেশনের সময় প্রিন্ট
করে, `UISON_FLASH=1` আবার আঁকা অঞ্চলগুলো হাইলাইট করে, আর `UISON_SCRIPT` / `UISON_CAPTURE` একটি সিনারিও চালায়
ও স্ক্রিনশট সেভ করে।

## রিপোজিটরির গঠন

```
crates/
  uison-json      JSON pull পার্সার ও ভ্যালু ট্রি, কমেন্ট, শেষে বাড়তি কমা ও কোটবিহীন কী সহ
  uison-core      জ্যামিতি, রং, ফন্ট, ইভেন্ট, Canvas ও Host ট্রেইট, App ট্রেইট
  uison-frame     প্রিমিটিভ ফ্রেম: পার্সিং, diff → damage, হিট টেস্টিং, সিনারিও রানার
  uison-ui        স্ক্রিন কম্পাইলার: টেমপ্লেট, বাইন্ডিং, থিম, লেআউট, রানটাইম, Rust DSL, lib/*.json
  uison-apple     macOS ও iOS-এর সাধারণ অংশ: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit ব্যাকএন্ড
  uison-ios       UIKit ব্যাকএন্ড
  uison-android   JNI ব্যাকএন্ড ও তার Java স্তর (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite ব্যাকএন্ড
  uison-linux     Wayland ও X11 ব্যাকএন্ড, Cairo + Pango, সবই dlopen-এর মাধ্যমে
  uison-web       WebAssembly ব্যাকএন্ড (crates/uison-web/uison.js)
  uison-c         C API ও C++ হেডার, স্ট্যাটিক ও ডায়নামিক লাইব্রেরি, CMake প্যাকেজ
  uison-gen       স্ক্রিন স্কিমা থেকে C++ উইজেট, Rust DSL ও এলিমেন্ট রেফারেন্স জেনারেট করে
  uison-derive    syn ছাড়া #[derive(Data)]
  uison           আমব্রেলা crate: বর্তমান প্ল্যাটফর্মের জন্য সবকিছু, uison::prelude; Rust পাঠ
examples/         Rust উদাহরণ (ওপরে দেখুন)
examples-c/       C উদাহরণ ও পাঠ, একটি CMake প্রজেক্ট
examples-cpp/     C++ উদাহরণ ও পাঠ, একটি CMake প্রজেক্ট
scenarios/        সব প্ল্যাটফর্মের সাধারণ টেস্ট সিনারিও
tools/            প্রতিটি প্ল্যাটফর্মের বিল্ড, রান ও টেস্ট স্ক্রিপ্ট; আসল মাউস ও কিবোর্ড ইনপুটের ড্রাইভার
web/              পৃষ্ঠা ও WebAssembly ব্যাকএন্ডের JavaScript অংশ
docs/             ইংরেজিতে ডকুমেন্টেশন; ru/ — একই জিনিস রুশ ভাষায়; readme/ — অন্যান্য ভাষায় এই README
```

## সীমাবদ্ধতা

- প্রতি প্রোগ্রামে একটি উইন্ডো; ড্র্যাগ-অ্যান্ড-ড্রপ নেই।
- টেক্সট ইনপুট লাতিন ও সিরিলিক এবং ফোনের অন-স্ক্রিন কিবোর্ড সমর্থন করে; ডেস্কটপে চীনা, জাপানি ও কোরিয়ানের
  কম্পোজড ইনপুট (IME) এখনও হয়নি, ডান থেকে বামে লেখা টেক্সটও নয়।
- এখনও অ্যাক্সেসিবিলিটি সমর্থন (স্ক্রিন রিডার) নেই।
- ব্রাউজারে থ্রেড নেই।
- ফ্রেমওয়ার্কটি নতুন: সংস্করণ 0.1, API বদলাতে পারে।

## অনুবাদ

ডকুমেন্টেশন ইংরেজিতে, আর `docs/ru/`-তে একই পৃষ্ঠাগুলো রুশ ভাষায় আছে, প্রতিটির ওপরে ভাষা বদলের সুইচসহ। এই
README-ও অনূদিত: `docs/readme/README.<code>.md`, প্রতিটি ভাষার জন্য একটি, প্রতিটির ওপরে ভাষার তালিকাসহ।
এখানে কোনো পরিবর্তন হলে তা অনুবাদগুলোতেও করতে হবে।

## লাইসেন্স

লাইসেন্স এখনও বেছে নেওয়া হয়নি।
