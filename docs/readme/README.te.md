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
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<b>🇮🇳 తెలుగు</b> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) అనేది Rustలో రాసిన చిన్న, వేగవంతమైన UI ఫ్రేమ్‌వర్క్; దీనికి బయటి డిపెండెన్సీలు ఏవీ
లేవు — కేవలం స్టాండర్డ్ లైబ్రరీ మాత్రమే, సొంత JSON పార్సర్‌తో సహా. మీరు స్క్రీన్‌ను ఎలిమెంట్‌ల ట్రీగా
వివరిస్తారు: JSON ఫైల్‌లో, Rust DSLతో, Flutter శైలిలో C++ విడ్జెట్‌లతో, లేదా నేరుగా C కోడ్ లోపల JSONగా.
లాజిక్ అనేది Rust, C లేదా C++లో సాధారణ కోడ్: హ్యాండ్లర్లు, డేటా, థ్రెడ్‌లు. విండో, లేఅవుట్, డ్రాయింగ్,
ఇన్‌పుట్‌లను uison చూసుకుంటుంది; అదే ప్రోగ్రామ్ macOS, Windows, Linux (Wayland, X11), iOS, Android,
బ్రౌజర్‌లలో నడుస్తుంది.

ప్రతి బ్యాకెండ్ చేతితో రాసిన FFI ద్వారా సిస్టమ్‌తో నేరుగా మాట్లాడుతుంది: AppKit, CoreText; Direct2D,
DirectWriteలతో Win32 (COM చేతితో); మొదటి నుంచి అమలు చేసిన Wayland ప్రోటోకాల్, X11; UIKit; JNI ద్వారా
Android Canvas; WebAssembly నుంచి 2D canvas. రిలీజ్ ప్రోగ్రామ్ సుమారు 0.7–0.9 MB ఉండే ఒకే ఫైల్, దానికి
సిస్టమ్ సొంత లైబ్రరీలు తప్ప మరేమీ అక్కర్లేదు; డెస్క్‌టాప్‌లో ఒక ఈవెంట్ సుమారు ఒక మిల్లీసెకనులో స్క్రీన్‌ను
చేరుతుంది.

![uisonతో నిర్మించిన ప్రోగ్రామ్‌లు: Studio (మెయిల్, సెట్టింగ్‌లు, చార్ట్‌లు), ఎలిమెంట్ గ్యాలరీ, Todo, Disk Usage మరియు ఒక చిన్న వెబ్ బ్రౌజర్](../img/showcase.jpg)

## ఇది ఎలా పనిచేస్తుంది

```text
స్క్రీన్ (JSON, Rust DSL, C++ విడ్జెట్‌లు, Cలో UISON_JSON)  +  డేటా  +  మీ భాషలో హ్యాండ్లర్లు
        │  బైండింగ్‌లు, టెంప్లేట్‌లు, థీమ్‌లు, లేఅవుట్ (uison-ui)
        ▼
ప్రిమిటివ్‌ల ఫ్రేమ్: rect, line, circle, tri, poly, text, image, clip, hit
        │  మునుపటి ఫ్రేమ్‌తో diff → మారినది మాత్రమే మళ్లీ గీయడం
        ▼
సిస్టమ్ బ్యాకెండ్: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

స్క్రీన్ కంపైల్ అయి ఒక *ఫ్రేమ్*గా మారుతుంది: కొన్ని రకాల ప్రిమిటివ్‌ల ఫ్లాట్ జాబితా — దీర్ఘచతురస్రాలు,
గీతలు, వృత్తాలు, బహుభుజాలు, ఒక్క వరుస టెక్స్ట్, చిత్రాలు, క్లిప్ ప్రాంతాలు, మౌస్, టచ్ కోసం కనిపించని హిట్
ఏరియాలు. ప్రతి కొత్త ఫ్రేమ్‌ను మునుపటి దానితో పోలుస్తారు, బ్యాకెండ్ మారిన ప్రాంతాలను మాత్రమే మళ్లీ పెయింట్
చేస్తుంది. బ్యాకెండ్ చిన్నదిగా ఉంటుంది, ఎందుకంటే అది ఈ ప్రిమిటివ్‌లను గీయడం, టెక్స్ట్‌ను కొలవడం, సిస్టమ్
ఈవెంట్‌లను uison ఈవెంట్‌లుగా మార్చడం మాత్రమే చేయాలి.

స్క్రీన్ భాషలో కోడ్ ఉండదు. టెక్స్ట్, ప్రాపర్టీలు డేటాకు బైండ్ అవుతాయి (`"Pressed {@n} times"`,
`"disabled": "!@form.valid"`), ఎలిమెంట్‌లు పారామీటర్లతో తిరిగి వాడుకోగల టెంప్లేట్‌లు, రంగులు, పరిమాణాలు
థీమ్ టోకెన్‌ల నుంచి వస్తాయి, ఈవెంట్‌లు పేరున్న యాక్షన్‌లను పిలుస్తాయి; వాటి హ్యాండ్లర్లు Rust, C లేదా C++లో
రాస్తారు. హ్యాండ్లర్ డేటాను మార్చినప్పుడు స్క్రీన్ తనను తాను మళ్లీ నిర్మించుకుంటుంది.

## హలో, వరల్డ్

మూడు భాషల్లోనూ అదే విండో — ఒక కౌంటర్, ఒక "+1" బటన్.

**Rust** — స్క్రీన్‌ను DSLతో నిర్మిస్తారు, హ్యాండ్లర్ బటన్‌పై ఒక క్లోజర్:

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

**C** — స్క్రీన్ నేరుగా కోడ్‌లో రాసిన JSON, హ్యాండ్లర్ ఒక ఫంక్షన్:

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

**C++** — Flutterలో లాగా విడ్జెట్‌లు, హ్యాండ్లర్ బటన్‌పై ఒక లాంబ్డా:

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

మీ సొంత ప్రాజెక్ట్‌తో మొదలుపెట్టడానికి [Rust](../rust/quickstart.md), [C](../c/quickstart.md) లేదా
[C++](../cpp/quickstart.md) క్విక్ స్టార్ట్ చూడండి.

## ఫీచర్లు

- **డిపెండెన్సీలు లేవు.** కోర్, JSON పార్సర్, లేఅవుట్, Wayland క్లయింట్, Linux సెట్టింగ్స్ పోర్టల్ కోసం
  D-Bus క్లయింట్, Windows కోసం COM బైండింగ్‌లు — అన్నీ ఇక్కడే, `std` పైన రాశారు. `Cargo.lock`లో
  వర్క్‌స్పేస్ సొంత crateలు మాత్రమే ఉంటాయి.
- **ఒకే ప్రోగ్రామ్, ఆరు ప్లాట్‌ఫామ్‌లు.** macOS, Windows (x64, ARM64), Linux (Wayland, X11; arm64,
  x86_64), iOS, Android (arm64-v8a, armeabi-v7a), బ్రౌజర్ (WebAssembly). ఫోన్‌లో అదే కోడ్‌కు టచ్, సేఫ్
  ఏరియా, ఆన్-స్క్రీన్ కీబోర్డ్, ఇనర్షియల్ స్క్రోలింగ్ లభిస్తాయి.
- **పాక్షిక రీడ్రా (partial redraw).** ఫ్రేమ్‌ల diff తీసి, దెబ్బతిన్న ప్రాంతాలను మాత్రమే మళ్లీ పెయింట్
  చేస్తారు; సన్నని గీతలను ప్రతి బ్యాకెండ్‌లో ఒకే విధంగా భౌతిక పిక్సెల్‌లకు సరిపెడతారు.
- **ఎలిమెంట్‌ల లైబ్రరీ**: బటన్‌లు, ఐకాన్ బటన్‌లు, 25 ఐకాన్‌లు, చెక్‌బాక్స్‌లు, రేడియో బటన్‌లు, స్విచ్‌లు,
  స్లైడర్‌లు, సెలెక్ట్‌లు, ట్యాబ్‌లు, సెలెక్షన్, క్లిప్‌బోర్డ్‌తో టెక్స్ట్ ఫీల్డ్‌లు, ప్రోగ్రెస్ బార్‌లు,
  కార్డ్‌లు, లిస్ట్ టైల్‌లు, బ్యాడ్జ్‌లు, అవతార్‌లు, టూల్‌టిప్‌లు, కాంటెక్స్ట్ మెనూలు, లింక్‌లు, చిత్రాలు.
  ఎలిమెంట్‌లు స్వయంగా JSON టెంప్లేట్‌లు (`lib/*.json`), మీ సొంత కాంపోనెంట్‌లు కూడా అదే విధంగా రాస్తారు.
- **Flutter శైలిలో లేఅవుట్**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, స్క్రోలింగ్, పదుల వేల
  వరుసల కోసం వర్చువలైజ్డ్ జాబితాలు, `Flow` — ఇన్‌లైన్ టెక్స్ట్ భాగాలతో కూడిన పేరా, వాటి సరిహద్దులను దాటి
  లైన్లు విరుగుతాయి.
- **బైండింగ్‌లు, స్టేట్**: డేటాలోకి పాత్‌లు (`@user.name`), టెక్స్ట్ ఇంటర్‌పొలేషన్, కాంపోనెంట్ లోకల్ స్టేట్
  (`@~open`), స్టేట్ ఆధారిత సెలెక్టర్లు (hover, pressed, focus), `switch`, కొన్ని ఫంక్షన్లు (`eq`, `not`,
  `len`, `ratio`…). స్ట్రక్ట్‌లు డేటాగా, డేటా తిరిగి స్ట్రక్ట్‌లుగా మారుతుంది: Rustలో `#[derive(Data)]`, Cలో
  ఫీల్డ్ టేబుల్‌లు, C++20లో మాక్రోలు లేని రిఫ్లెక్షన్.
- **థీమ్‌లు**: లైట్, డార్క్, "సిస్టమ్‌ను అనుసరించు" — మీరు ఓవర్‌రైడ్ చేయగల టోకెన్‌లతో నిర్మించినవి.
- **మీ సొంత డ్రాయింగ్**: `Canvas` నోడ్ సాధారణ ఎలిమెంట్‌ల పక్కనే తన దీర్ఘచతురస్రంలో ప్రిమిటివ్‌లు రాయడానికి
  మీ ఫంక్షన్‌ను పిలుస్తుంది.
- **థ్రెడ్‌లు**: వర్కర్ థ్రెడ్ ఫలితాలను UI థ్రెడ్‌కు తిరిగి పంపుతుంది (`post`), బ్రౌజర్ తప్ప అన్ని
  ప్లాట్‌ఫామ్‌లలో.
- **లైవ్ రీలోడ్** డీబగ్ బిల్డ్‌లలో: `UISON_RELOAD=folder`తో స్క్రీన్ JSON ఫైళ్లను సేవ్ చేసినప్పుడు అవి మళ్లీ
  చదవబడతాయి.
- **విండో లేకుండా టెస్టింగ్**: ఒక సినారియో (`UISON_SCRIPT=scenario.json`) ఎలిమెంట్ `key` ద్వారా ట్యాప్
  చేస్తుంది, టైప్ చేస్తుంది, స్క్రోల్ చేస్తుంది, స్క్రీన్‌షాట్‌లు తీస్తుంది; అవే సినారియోలు ప్రతి
  ప్లాట్‌ఫామ్‌లో, headless Chromeలో, iOS సిమ్యులేటర్‌లో, ఫోన్‌లో నడుస్తాయి.
- **[ఏజెంట్ మోడ్](../agents.md)**: LLM ఏజెంట్ స్క్రీన్‌షాట్‌లు లేకుండా ప్రోగ్రామ్‌ను వాడుతుంది. అది స్క్రీన్‌ను అర్థంతో చదువుతుంది (పాత్రలు, పేర్లు, స్థితులు, key)
  మనిషి ఇన్‌పుట్ వెళ్లే దారిలోనే క్లిక్, టైప్, ఎంపిక చేస్తుంది — JSON లైన్‌లతో (`--agent`) లేదా MCP సర్వర్‌గా (`--mcp`).
- **C, C++ API**: `uison.h` (C99), `uison_kit.h` (C కోసం సహాయకాలు), `uison.hpp` (C++17, C++20లో అదనపు
  సౌకర్యాలు), `uison_widgets.hpp` (Rust DSL ఉత్పత్తి అయ్యే అదే స్కీమా నుంచి జనరేట్ అయిన C++20 విడ్జెట్‌లు);
  స్టాటిక్, డైనమిక్ లైబ్రరీలు, CMake ప్యాకేజ్, `pkg-config`. Emscriptenతో C, C++లను బ్రౌజర్ కోసం కూడా బిల్డ్
  చేయవచ్చు: `em++ main.cpp libuison.a -o index.html`.

## ప్లాట్‌ఫామ్‌లు

| ప్లాట్‌ఫామ్ | బ్యాకెండ్ | ఎక్కడ పరీక్షించారు |
|---|---|---|
| macOS | Objective-C రన్‌టైమ్ ద్వారా AppKit, CoreGraphics, CoreText, ImageIO | Apple Silicon Mac — ప్రధాన డెవలప్‌మెంట్ యంత్రం |
| iOS | UIKit, CADisplayLink, macOSలో ఉన్న అదే CoreGraphics/CoreText కోడ్ | సిమ్యులేటర్, iPhone 12 mini |
| Android | చేతితో JNI, `android.graphics.Canvas`, సుమారు 300 లైన్ల Java పొర; Gradle లేకుండా APK బిల్డ్ | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite, WIC, చేతితో COM; ప్రతి మానిటర్‌కు DPI, డార్క్ టైటిల్ బార్ | Windows 11 PC, 100/125/150 % వద్ద, వేర్వేరు స్కేల్‌లు ఉన్న రెండు మానిటర్లు |
| Linux | Wayland ప్రోటోకాల్ సొంత కోడ్, X11 (xcb), Cairo + Pango, xkbcommon — అన్నీ `dlopen` ద్వారా, కాబట్టి బైనరీ libc పై మాత్రమే ఆధారపడుతుంది | headless Mutter, KWin, Sway, Weston, X11 (arm64, x86_64); వర్చువల్ మెషీన్‌లో GNOMEతో Ubuntu 24.04 |
| బ్రౌజర్ | wasm-bindgen లేకుండా `wasm32-unknown-unknown`, canvas 2D; Emscripten ద్వారా C/C++ | Chrome (headless Chrome కూడా) |

ఇంకా పరీక్షించనివి: Windowsలో నిజమైన టచ్‌స్క్రీన్, ARMపై Windows (బిల్డ్ అవుతుంది కానీ నడపలేదు), నిజమైన
హార్డ్‌వేర్‌పై Linux, KDE, GNOMEలో మౌస్, కీబోర్డ్ ఇన్‌పుట్, Safari, Firefox.

## పరిమాణం, వేగం

రిలీజ్ బిల్డ్‌లు (`opt-level = "s"`, LTO, `panic = "abort"`, strip చేసినవి). పరిమాణాలు ప్రాజెక్ట్
పెరుగుతున్నప్పుడు దాని సొంత కొలతల నుంచి తీసుకున్నవి, కాబట్టి తాజా బిల్డ్‌లు కొన్ని శాతం తేడా ఉండవచ్చు.

| ప్రోగ్రామ్ | పరిమాణం |
|---|---|
| పాఠం 1 (పైన ఉన్న కౌంటర్), Rust, macOS | ~0.7 MB |
| అదే C / C++లో `libuison.a`తో, macOS | 672 KB / 816 KB |
| Studio — ప్రధాన డెమో (2,000 సందేశాలతో మెయిల్, కాన్‌బాన్, కాంటాక్ట్‌లు, చార్ట్‌లు, ఫారమ్‌లు), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, Visual C++ Redistributable అవసరం లేదు |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, బ్రౌజర్ (`.wasm`, gzip) | ~220 KB |

| కొలత | ఫలితం |
|---|---|
| ఈవెంట్ → పెయింట్ అయిన విండో, macOS | ~0.8 ms, అందులో ~0.2 ms uison సొంత పని |
| టచ్ → ఫ్రేమ్, iPhone 12 mini | 3.6 ms; స్థిరమైన 60 fpsతో జాబితా స్క్రోలింగ్ |
| Studio పూర్తి రీపెయింట్, iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| మౌస్ ఈవెంట్ → ఫ్రేమ్, Windows (Iris Xe, 60 Hz) | p50 1.4–1.7 ms |
| ఈవెంట్ → ఫ్రేమ్, Chrome | మీడియన్ 2.2 ms |
| లాంచ్ → మొదటి ఫ్రేమ్ | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, బ్రౌజర్ 24–42 ms |

egui ప్రోగ్రామ్‌కు పోర్ట్ అయిన [Disk Usage](../../examples/disk-usage/README.md) నేరుగా పోలిక ఇస్తుంది:
macOS arm64లో బైనరీ 5.1 MBకి బదులు 857 KB, `Cargo.lock`లో 362 crateలకు బదులు ఒక్క డిపెండెన్సీ కూడా లేదు,
స్కానింగ్ వేగం, మెమరీ వినియోగం అలాగే ఉన్నాయి.

## డాక్యుమెంటేషన్

డాక్యుమెంటేషన్ ఇంగ్లీష్‌లో ఉంది, ప్రతి పేజీకి ఒక [రష్యన్ వెర్షన్](../ru/README.md) కూడా ఉంది.

- [భాషను ఎంచుకోండి](../README.md) — ఎక్కడ మొదలుపెట్టాలి.
- పాఠాల సిరీస్‌లు, ప్రతిదాన్నీ విడిగా చదవవచ్చు, మీ సిస్టమ్ సిద్ధం చేయడం నుంచి థ్రెడ్‌లు, మీ సొంత ఫ్రేమ్
  వరకు: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md). ప్రధాన పాఠాలు మూడు
  భాషల్లోనూ అదే ప్రోగ్రామ్, వాటి స్క్రీన్‌షాట్‌లు పిక్సెల్‌కు పిక్సెల్ సరిపోలుతాయి.
- [స్క్రీన్ భాష](../screens/README.md): [బైండింగ్‌లు, విలువలు](../screens/bindings.md),
  [లేఅవుట్](../screens/layout.md), [ఎలిమెంట్‌లు](../screens/elements.md), [థీమ్‌లు](../screens/themes.md).
- [iOS, Android](../mobile.md), [బ్రౌజర్ (WebAssembly)](../wasm.md).

## ఉదాహరణలు

| ఉదాహరణ | ఏమి చూపిస్తుంది |
|---|---|
| `studio` | ప్రధాన డెమో: 2,000 సందేశాలతో మెయిల్, కాన్‌బాన్, కాంటాక్ట్‌లు, యానిమేటెడ్ చార్ట్‌లు, ఒక ఫారమ్, సెట్టింగ్‌లు; 700 pt కంటే తక్కువ వెడల్పుకు ఫోన్ లేఅవుట్ |
| `gallery` | లైబ్రరీలోని ప్రతి ఎలిమెంట్ |
| `disk-usage` | egui యాప్ పోర్ట్: `Canvas` నోడ్‌లో గీసిన సన్‌బర్స్ట్ చార్ట్, uison స్క్రీన్‌గా ప్యానెల్‌లు, మెనూలు, `std` పై సమాంతర స్కానర్ |
| `browser` | చిన్న వెబ్ బ్రౌజర్: సొంత HTML, CSS పార్సర్, క్యాస్కేడ్, HTTP, సిస్టమ్ TLS ద్వారా HTTPS; పేజీ uison స్క్రీన్‌గా మారుతుంది |
| `todo` | Rust DSLలో టు-డూ జాబితా |
| `list10k` | 10,000 వరుసల వర్చువలైజ్డ్ జాబితా |
| `form2`, `settings` | ఫారమ్‌లు, సెట్టింగ్‌ల స్క్రీన్‌లు |
| `demo`, `scroll`, `uibench` | చేతితో ప్రిమిటివ్‌లు, స్క్రోలింగ్, బెంచ్‌మార్క్ |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C, C++ ఉదాహరణలు [`examples-c`](../../examples-c), [`examples-cpp`](../../examples-cpp)లో ఉన్నాయి,
ప్రతిదీ తనదైన CMake ప్రాజెక్ట్.

## బిల్డ్, టెస్ట్

మీకు stable Rust కావాలి; macOS, Windows (MSVC టూల్‌చెయిన్‌తో), Linuxలో ఇంకేమీ అక్కర్లేదు (`-dev`
ప్యాకేజీలు అవసరం లేదు: సిస్టమ్ లైబ్రరీలు రన్‌టైమ్‌లో లోడ్ అవుతాయి). ప్రతి సిస్టమ్‌ను సిద్ధం చేయడం ప్రతి
సిరీస్‌లో పాఠం 0 ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windowsలో స్క్రిప్ట్‌లు `tools\*.ps1`: `tools\win.ps1 studio` బిల్డ్ చేసి, సినారియో నడిపి, స్క్రీన్‌షాట్‌లు
సేవ్ చేస్తుంది; `tools\c-dist.ps1 -Test` C/C++ ప్యాకేజ్‌ను బిల్డ్ చేసి పరీక్షిస్తుంది.

డీబగ్గింగ్ కోసం బ్యాకెండ్ స్విచ్‌లు, అన్నిచోట్లా ఒకేలా: `UISON_STATS=1` ఈవెంట్‌లు, ఫ్రేమ్‌లు, యానిమేషన్
సమయాలను ప్రింట్ చేస్తుంది, `UISON_FLASH=1` మళ్లీ పెయింట్ అయిన ప్రాంతాలను హైలైట్ చేస్తుంది, `UISON_SCRIPT` /
`UISON_CAPTURE` సినారియో నడిపి స్క్రీన్‌షాట్‌లు సేవ్ చేస్తాయి.

## రిపాజిటరీ నిర్మాణం

```
crates/
  uison-json      JSON pull పార్సర్, విలువల ట్రీ; కామెంట్‌లు, చివరి అదనపు కామాలు, కోట్స్ లేని కీలతో
  uison-core      జ్యామితి, రంగులు, ఫాంట్‌లు, ఈవెంట్‌లు, Canvas, Host ట్రెయిట్‌లు, App ట్రెయిట్
  uison-frame     ప్రిమిటివ్ ఫ్రేమ్: పార్సింగ్, diff → damage, హిట్ టెస్టింగ్, సినారియో రన్నర్
  uison-ui        స్క్రీన్ కంపైలర్: టెంప్లేట్‌లు, బైండింగ్‌లు, థీమ్‌లు, లేఅవుట్, రన్‌టైమ్, Rust DSL, lib/*.json
  uison-apple     macOS, iOSలకు ఉమ్మడి: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit బ్యాకెండ్
  uison-ios       UIKit బ్యాకెండ్
  uison-android   JNI బ్యాకెండ్, దాని Java పొర (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite బ్యాకెండ్
  uison-linux     Wayland, X11 బ్యాకెండ్, Cairo + Pango, అన్నీ dlopen ద్వారా
  uison-web       WebAssembly బ్యాకెండ్ (crates/uison-web/uison.js)
  uison-c         C API, C++ హెడర్‌లు, స్టాటిక్, డైనమిక్ లైబ్రరీ, CMake ప్యాకేజ్
  uison-gen       స్క్రీన్ స్కీమా నుంచి C++ విడ్జెట్‌లు, Rust DSL, ఎలిమెంట్ రిఫరెన్స్‌ను జనరేట్ చేస్తుంది
  uison-derive    syn లేకుండా #[derive(Data)]
  uison           అంబ్రెల్లా crate: ప్రస్తుత ప్లాట్‌ఫామ్ కోసం అన్నీ, uison::prelude; Rust పాఠాలు
examples/         Rust ఉదాహరణలు (పైన చూడండి)
examples-c/       C ఉదాహరణలు, పాఠాలు; CMake ప్రాజెక్ట్
examples-cpp/     C++ ఉదాహరణలు, పాఠాలు; CMake ప్రాజెక్ట్
scenarios/        అన్ని ప్లాట్‌ఫామ్‌లకు ఉమ్మడి టెస్ట్ సినారియోలు
tools/            ప్రతి ప్లాట్‌ఫామ్ కోసం బిల్డ్, రన్, టెస్ట్ స్క్రిప్ట్‌లు; నిజమైన మౌస్, కీబోర్డ్ ఇన్‌పుట్ డ్రైవర్లు
web/              పేజీ, WebAssembly బ్యాకెండ్ JavaScript భాగం
docs/             ఇంగ్లీష్‌లో డాక్యుమెంటేషన్; ru/ — అదే రష్యన్‌లో; readme/ — ఈ README ఇతర భాషల్లో
```

## పరిమితులు

- ఒక ప్రోగ్రామ్‌కు ఒకే విండో; డ్రాగ్-అండ్-డ్రాప్ లేదు.
- టెక్స్ట్ ఇన్‌పుట్ లాటిన్, సిరిలిక్ లిపులను, ఫోన్‌ల ఆన్-స్క్రీన్ కీబోర్డ్‌లను కవర్ చేస్తుంది; డెస్క్‌టాప్‌లో
  చైనీస్, జపనీస్, కొరియన్ కోసం కంపోజ్డ్ ఇన్‌పుట్ (IME) ఇంకా చేయలేదు, కుడి నుంచి ఎడమకు రాసే టెక్స్ట్ కూడా
  లేదు.
- ఇంకా యాక్సెసిబిలిటీ సపోర్ట్ (స్క్రీన్ రీడర్లు) లేదు.
- బ్రౌజర్‌లో థ్రెడ్‌లు లేవు.
- ఫ్రేమ్‌వర్క్ కొత్తది: వెర్షన్ 0.1, API మారవచ్చు.

## అనువాదాలు

డాక్యుమెంటేషన్ ఇంగ్లీష్‌లో ఉంది, `docs/ru/`లో అవే పేజీలు రష్యన్‌లో ఉన్నాయి, ప్రతి పేజీ పైభాగంలో భాష మార్చే
స్విచ్ ఉంది. ఈ README కూడా అనువదించబడింది: `docs/readme/README.<code>.md`, ఒక్కో భాషకు ఒకటి, ప్రతిదాని
పైభాగంలో భాషల జాబితాతో. ఇక్కడ చేసే మార్పు అనువాదాల్లోనూ చేయాలి.

## లైసెన్స్

లైసెన్స్ ఇంకా ఎంచుకోలేదు.
