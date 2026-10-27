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
<b>🇩🇪 Deutsch</b> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) ist ein kleines, schnelles UI-Framework in Rust ganz ohne
Abhängigkeiten – nur die Standardbibliothek, bis hin zum eigenen JSON-Parser. Ein Screen
wird als Baum von Elementen beschrieben: in einer JSON-Datei, mit einer Rust-DSL, mit
C++-Widgets im Stil von Flutter oder als JSON direkt im C-Code. Die Logik ist gewöhnlicher
Code in Rust, C oder C++: Handler, Daten, Threads. Um Fenster, Layout, Zeichnen und Eingabe
kümmert sich uison, und dasselbe Programm läuft unter macOS, Windows, Linux (Wayland und X11),
iOS, Android und im Browser.

Jedes Backend spricht direkt mit dem System, über handgeschriebenes FFI: AppKit und CoreText,
Win32 mit Direct2D und DirectWrite (COM von Hand), das von Grund auf implementierte
Wayland-Protokoll und X11, UIKit, das Canvas von Android über JNI und ein 2D-Canvas aus
WebAssembly. Ein Release-Programm ist eine einzige Datei von etwa 0,7–0,9 MB, die nichts
außer den Bibliotheken des Systems braucht, und ein Ereignis erreicht auf dem Desktop in
etwa einer Millisekunde den Bildschirm.

![Mit uison gebaute Programme: Studio (Mail, Einstellungen, Diagramme), die Elementgalerie, Todo, Disk Usage und ein kleiner Webbrowser](../img/showcase.jpg)

## Wie es funktioniert

```text
Screen (JSON, Rust-DSL, C++-Widgets, UISON_JSON in C)  +  Daten  +  Handler in Ihrer Sprache
        │  Bindings, Templates, Themes, Layout (uison-ui)
        ▼
Frame aus Primitiven: rect, line, circle, tri, poly, text, image, clip, hit
        │  Diff gegen den vorigen Frame → nur Geändertes neu zeichnen
        ▼
System-Backend: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Ein Screen wird zu einem *Frame* kompiliert: einer flachen Liste weniger Arten von
Primitiven – Rechtecke, Linien, Kreise, Polygone, einzeilige Texte, Bilder, Clip-Bereiche
und unsichtbare Trefferflächen für Maus und Touch. Jeder neue Frame wird mit dem vorigen
verglichen, und das Backend zeichnet nur die geänderten Bereiche neu. Ein Backend bleibt
klein, weil es nur diese Primitive zeichnen, Text messen und die Ereignisse des Systems in
die von uison übersetzen muss.

Die Screen-Sprache enthält keinen Code. Texte und Eigenschaften sind an die Daten gebunden
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), Elemente sind wiederverwendbare
Templates mit Parametern, Farben und Größen kommen aus Theme-Tokens, und Ereignisse rufen
benannte Aktionen auf, deren Handler in Rust, C oder C++ geschrieben sind. Ändert ein
Handler die Daten, baut sich der Screen selbst neu auf.

## Hello, world

Dasselbe Fenster – ein Zähler und ein „+1“-Button – in den drei Sprachen.

**Rust** – der Screen wird mit einer DSL gebaut, der Handler ist eine Closure am Button:

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

**C** – der Screen ist JSON direkt im Code, der Handler ist eine Funktion:

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

**C++** – Widgets wie in Flutter, der Handler ist ein Lambda am Button:

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

Für ein eigenes Projekt siehe den Schnellstart für
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) oder [C++](../cpp/quickstart.md).

## Funktionen

- **Keine Abhängigkeiten.** Der Kern, der JSON-Parser, das Layout, der Wayland-Client, der
  D-Bus-Client für das Linux-Settings-Portal und die COM-Bindings für Windows sind alle hier
  geschrieben, auf Basis von `std`. `Cargo.lock` enthält nur die eigenen Crates des Workspace.
- **Ein Programm, sechs Plattformen.** macOS, Windows (x64 und ARM64), Linux (Wayland und X11,
  arm64 und x86_64), iOS, Android (arm64-v8a und armeabi-v7a) und der Browser (WebAssembly).
  Auf dem Smartphone bekommt derselbe Code Touch, eine Safe Area, die Bildschirmtastatur und
  Scrollen mit Trägheit.
- **Partielles Neuzeichnen.** Frames werden per Diff verglichen und nur beschädigte Bereiche
  neu gezeichnet; dünne Linien werden in jedem Backend gleich an physischen Pixeln ausgerichtet.
- **Eine Elementbibliothek**: Buttons, Icon-Buttons und 25 Icons, Checkboxen, Radiobuttons,
  Schalter, Slider, Auswahllisten, Tabs, Textfelder mit Auswahl und Zwischenablage,
  Fortschrittsbalken, Karten, Listeneinträge, Badges, Avatare, Tooltips, Kontextmenüs, Links
  und Bilder. Die Elemente sind selbst JSON-Templates (`lib/*.json`), und eigene Komponenten
  schreibt man genauso.
- **Layout im Geist von Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, Flex, Scrollen,
  virtualisierte Listen für Zehntausende Zeilen und `Flow` – ein Absatz aus Inline-Textstücken
  mit Umbruch über deren Grenzen hinweg.
- **Bindings und Zustand**: Pfade in die Daten (`@user.name`), Textinterpolation, lokaler
  Zustand einer Komponente (`@~open`), Selektoren nach Zustand (hover, pressed, focus),
  `switch` und einige Funktionen (`eq`, `not`, `len`, `ratio`…). Structs werden auf Daten
  abgebildet und zurück: `#[derive(Data)]` in Rust, Feldtabellen in C, Reflection ohne Makros
  in C++20.
- **Themes**: hell, dunkel und „wie das System“, aufgebaut aus überschreibbaren Tokens.
- **Eigenes Zeichnen**: Ein `Canvas`-Knoten ruft Ihre Funktion auf, die Primitive in sein
  Rechteck schreibt – neben gewöhnlichen Elementen.
- **Threads**: Ein Worker-Thread schickt Ergebnisse an den UI-Thread zurück (`post`), auf
  allen Plattformen außer dem Browser.
- **Live Reload** in Debug-Builds: Mit `UISON_RELOAD=folder` werden die JSON-Dateien des
  Screens beim Speichern neu eingelesen.
- **Testen ohne Fenster**: Ein Szenario (`UISON_SCRIPT=scenario.json`) tippt, gibt Text ein,
  scrollt und macht Screenshots anhand des `key` der Elemente; dieselben Szenarien laufen
  auf jeder Plattform, in Headless Chrome, im iOS-Simulator und auf dem Smartphone.
- **C- und C++-API**: `uison.h` (C99), `uison_kit.h` (Helfer für C), `uison.hpp` (C++17,
  Komfortfunktionen in C++20) und `uison_widgets.hpp` (C++20-Widgets, generiert aus demselben
  Schema wie die Rust-DSL); statische und dynamische Bibliotheken, ein CMake-Paket und
  `pkg-config`. C und C++ lassen sich mit Emscripten auch für den Browser bauen:
  `em++ main.cpp libuison.a -o index.html`.

## Plattformen

| Plattform | Backend | geprüft auf |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO über die Objective-C-Runtime | Mac mit Apple Silicon – die Hauptentwicklungsmaschine |
| iOS | UIKit, CADisplayLink, derselbe CoreGraphics/CoreText-Code wie unter macOS | Simulator und iPhone 12 mini |
| Android | JNI von Hand, `android.graphics.Canvas`, eine Java-Schicht von ~300 Zeilen; APK ohne Gradle gebaut | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite und WIC, COM von Hand; DPI pro Monitor, dunkle Titelleiste | Windows-11-PC bei 100/125/150 % und zwei Monitore mit unterschiedlicher Skalierung |
| Linux | eigener Code für das Wayland-Protokoll und X11 (xcb), Cairo + Pango, xkbcommon – alles über `dlopen`, daher hängt das Binary nur von libc ab | Mutter, KWin, Sway, Weston und X11 headless (arm64 und x86_64); Ubuntu 24.04 mit GNOME in einer virtuellen Maschine |
| Browser | `wasm32-unknown-unknown` ohne wasm-bindgen, canvas 2D; C/C++ über Emscripten | Chrome (und Headless Chrome) |

Noch nicht geprüft: ein echter Touchscreen unter Windows, Windows on ARM (baut, wurde aber
nicht ausgeführt), Linux auf echter Hardware und KDE, Maus- und Tastatureingabe in GNOME,
Safari und Firefox.

## Größe und Geschwindigkeit

Release-Builds (`opt-level = "s"`, LTO, `panic = "abort"`, gestrippt). Die Größen stammen
aus den eigenen Messungen des Projekts während seiner Entwicklung, die neuesten Builds
können also um einige Prozent abweichen.

| Programm | Größe |
|---|---|
| Lektion 1 (der Zähler oben), Rust, macOS | ~0,7 MB |
| dasselbe in C / C++ mit `libuison.a`, macOS | 672 KB / 816 KB |
| Studio – die Haupt-Demo (Mail mit 2.000 Nachrichten, Kanban, Kontakte, Diagramme, Formulare), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, kein Visual C++ Redistributable nötig |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, Browser (`.wasm`, gzip) | ~220 KB |

| Messung | Ergebnis |
|---|---|
| Ereignis → gezeichnetes Fenster, macOS | ~0,8 ms, davon ~0,2 ms Arbeit von uison selbst |
| Touch → Frame, iPhone 12 mini | 3,6 ms; Scrollen einer Liste mit gleichmäßigen 60 fps |
| komplettes Neuzeichnen von Studio, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| Mausereignis → Frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| Ereignis → Frame, Chrome | Median 2,2 ms |
| Start → erster Frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, Browser 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), eine Portierung eines egui-Programms,
erlaubt einen direkten Vergleich: Unter macOS arm64 ist das Binary 857 KB statt 5,1 MB groß,
ohne Abhängigkeiten statt 362 Crates in `Cargo.lock`, bei gleicher Scan-Geschwindigkeit und
gleichem Speicherverbrauch.

## Dokumentation

Die Dokumentation ist auf Englisch, mit einer [russischen Version](../ru/README.md) jeder Seite.

- [Sprache wählen](../README.md) – wo man anfängt.
- Lektionsreihen, jede für sich lesbar, von der Einrichtung des Systems bis zu Threads und
  einem eigenen Frame: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Die Kernlektionen sind in allen drei Sprachen dasselbe Programm, und ihre Screenshots
  stimmen pixelgenau überein.
- [Die Screen-Sprache](../screens/README.md): [Bindings und Werte](../screens/bindings.md),
  [Layout](../screens/layout.md), [Elemente](../screens/elements.md),
  [Themes](../screens/themes.md).
- [iOS und Android](../mobile.md), [der Browser (WebAssembly)](../wasm.md).

## Beispiele

| Beispiel | was es zeigt |
|---|---|
| `studio` | die Haupt-Demo: Mail mit 2.000 Nachrichten, Kanban, Kontakte, animierte Diagramme, ein Formular, Einstellungen; ein Smartphone-Layout unter 700 pt |
| `gallery` | alle Elemente der Bibliothek |
| `disk-usage` | Portierung einer egui-App: ein Sunburst-Diagramm in einem `Canvas`-Knoten, Panels und Menüs als uison-Screen, ein paralleler Scanner auf Basis von `std` |
| `browser` | ein kleiner Webbrowser: eigener HTML- und CSS-Parser samt Kaskade, HTTP und HTTPS über das TLS des Systems; eine Seite wird zu einem uison-Screen |
| `todo` | eine To-do-Liste in der Rust-DSL |
| `list10k` | eine virtualisierte Liste mit 10.000 Zeilen |
| `form2`, `settings` | Formulare und Einstellungs-Screens |
| `demo`, `scroll`, `uibench` | Primitive von Hand, Scrollen, ein Benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C- und C++-Beispiele liegen in [`examples-c`](../../examples-c) und [`examples-cpp`](../../examples-cpp),
jeweils ein eigenes CMake-Projekt.

## Bauen und Testen

Sie brauchen stabiles Rust; sonst nichts unter macOS, Windows (mit der MSVC-Toolchain) und
Linux (keine `-dev`-Pakete: Systembibliotheken werden zur Laufzeit geladen). Die Einrichtung
jedes Systems ist Lektion 0 jeder Reihe ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Unter Windows heißen die Skripte `tools\*.ps1`: `tools\win.ps1 studio` baut, führt ein
Szenario aus und speichert Screenshots, `tools\c-dist.ps1 -Test` baut und testet das
C/C++-Paket.

Backend-Schalter zum Debuggen, überall gleich: `UISON_STATS=1` gibt Zeiten für Ereignisse,
Frames und Animation aus, `UISON_FLASH=1` hebt neu gezeichnete Bereiche hervor, und
`UISON_SCRIPT` / `UISON_CAPTURE` führen ein Szenario aus und speichern Screenshots.

## Aufbau

```
crates/
  uison-json      JSON-Pull-Parser und Werte-Baum, mit Kommentaren, nachgestellten Kommas und Schlüsseln ohne Anführungszeichen
  uison-core      Geometrie, Farben, Schriften, Ereignisse, die Traits Canvas und Host, der Trait App
  uison-frame     der Primitiv-Frame: Parsen, Diff → Damage, Hit-Testing, der Szenario-Runner
  uison-ui        der Screen-Compiler: Templates, Bindings, Themes, Layout, Runtime, Rust-DSL, lib/*.json
  uison-apple     gemeinsam für macOS und iOS: Objective-C-FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit-Backend
  uison-ios       UIKit-Backend
  uison-android   JNI-Backend und seine Java-Schicht (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite-Backend
  uison-linux     Wayland- und X11-Backend, Cairo + Pango, alles über dlopen
  uison-web       WebAssembly-Backend (crates/uison-web/uison.js)
  uison-c         die C-API und C++-Header, statische und dynamische Bibliothek, CMake-Paket
  uison-gen       generiert die C++-Widgets, die Rust-DSL und die Elementreferenz aus dem Screen-Schema
  uison-derive    #[derive(Data)] ohne syn
  uison           das Dach-Crate: alles für die aktuelle Plattform, uison::prelude; die Rust-Lektionen
examples/         Rust-Beispiele (siehe oben)
examples-c/       C-Beispiele und -Lektionen, ein CMake-Projekt
examples-cpp/     C++-Beispiele und -Lektionen, ein CMake-Projekt
scenarios/        Testszenarien, gemeinsam für alle Plattformen
tools/            Skripte zum Bauen, Starten und Testen für jede Plattform; Treiber für echte Maus- und Tastatureingabe
web/              die Seite und die JavaScript-Seite des WebAssembly-Backends
docs/             Dokumentation auf Englisch; ru/, dasselbe auf Russisch; readme/, diese README in anderen Sprachen
```

## Einschränkungen

- Ein Fenster pro Programm; kein Drag-and-drop.
- Die Texteingabe deckt lateinische und kyrillische Schrift sowie die Bildschirmtastaturen
  von Smartphones ab; zusammengesetzte Eingabe für Chinesisch, Japanisch und Koreanisch (IME)
  auf dem Desktop ist nicht umgesetzt, ebenso wenig Text von rechts nach links.
- Noch keine Barrierefreiheit (Screenreader).
- Keine Threads im Browser.
- Das Framework ist jung: Version 0.1, die API kann sich ändern.

## Übersetzungen

Die Dokumentation ist auf Englisch, und `docs/ru/` enthält dieselben Seiten auf Russisch,
jeweils mit einem Sprachumschalter oben. Auch diese README ist übersetzt:
`docs/readme/README.<code>.md`, eine pro Sprache, jeweils mit der Liste der Sprachen oben.
Eine Änderung hier gehört auch in die Übersetzungen.

## Lizenz

Die Lizenz ist noch nicht festgelegt.
