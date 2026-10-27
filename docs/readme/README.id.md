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
<b>🇮🇩 Bahasa Indonesia</b> ·
<a href="README.de.md">🇩🇪 Deutsch</a> ·
<a href="README.ja.md">🇯🇵 日本語</a> ·
<a href="README.mr.md">🇮🇳 मराठी</a> ·
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<a href="README.tr.md">🇹🇷 Türkçe</a> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) adalah framework UI kecil dan cepat yang ditulis dalam Rust tanpa
dependensi sama sekali — hanya pustaka standar, bahkan parser JSON-nya pun dibuat sendiri.
Anda mendeskripsikan sebuah layar sebagai pohon elemen: dalam file JSON, dengan DSL Rust,
dengan widget C++ ala Flutter, atau sebagai JSON langsung di dalam kode C. Logikanya berupa
kode biasa dalam Rust, C, atau C++: handler, data, thread. uison mengurus jendela, layout,
penggambaran, dan input, dan program yang sama berjalan di macOS, Windows, Linux (Wayland dan
X11), iOS, Android, dan di browser.

Setiap backend berkomunikasi langsung dengan sistem melalui FFI yang ditulis tangan: AppKit
dan CoreText, Win32 dengan Direct2D dan DirectWrite (COM ditulis tangan), protokol Wayland
yang diimplementasikan dari nol dan X11, UIKit, Canvas Android lewat JNI, serta canvas 2D dari
WebAssembly. Program release berupa satu file berukuran sekitar 0,7–0,9 MB yang tidak
membutuhkan apa pun selain pustaka bawaan sistem, dan sebuah event sampai ke layar dalam
sekitar satu milidetik di desktop.

![Program yang dibuat dengan uison: Studio (surel, pengaturan, grafik), galeri elemen, Todo, Disk Usage, dan peramban web kecil](../img/showcase.jpg)

## Cara kerjanya

```text
layar (JSON, DSL Rust, widget C++, UISON_JSON di C)  +  data  +  handler dalam bahasa Anda
        │  binding, template, tema, layout (uison-ui)
        ▼
frame berisi primitif: rect, line, circle, tri, poly, text, image, clip, hit
        │  diff terhadap frame sebelumnya → gambar ulang hanya yang berubah
        ▼
backend sistem: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Sebuah layar dikompilasi menjadi *frame*: daftar datar berisi beberapa jenis primitif —
persegi panjang, garis, lingkaran, poligon, teks satu baris, gambar, area clip, dan area hit
tak terlihat untuk mouse dan sentuhan. Setiap frame baru dibandingkan dengan frame
sebelumnya, dan backend hanya menggambar ulang area yang berubah. Backend bisa kecil karena
ia hanya perlu menggambar primitif-primitif ini, mengukur teks, dan mengubah event sistem
menjadi event uison.

Bahasa layar tidak berisi kode. Teks dan properti di-binding ke data
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), elemen adalah template yang bisa
dipakai ulang dengan parameter, warna dan ukuran berasal dari token tema, dan event memanggil
aksi bernama yang handler-nya ditulis dalam Rust, C, atau C++. Ketika handler mengubah data,
layar membangun ulang dirinya sendiri.

## Halo, dunia

Jendela yang sama — sebuah penghitung dan tombol "+1" — dalam tiga bahasa.

**Rust** — layar dibangun dengan DSL, handler-nya berupa closure pada tombol:

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

**C** — layar berupa JSON yang ditulis langsung di kode, handler-nya berupa fungsi:

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

**C++** — widget seperti di Flutter, handler-nya berupa lambda pada tombol:

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

Untuk memulai proyek Anda sendiri, lihat panduan quick start (dalam bahasa Inggris) untuk
[Rust](../rust/quickstart.md), [C](../c/quickstart.md), atau [C++](../cpp/quickstart.md).

## Fitur

- **Tanpa dependensi.** Inti, parser JSON, layout, klien Wayland, klien D-Bus untuk portal
  pengaturan Linux, dan binding COM untuk Windows semuanya ditulis di sini, di atas `std`.
  `Cargo.lock` hanya berisi crate milik workspace itu sendiri.
- **Satu program, enam platform.** macOS, Windows (x64 dan ARM64), Linux (Wayland dan X11,
  arm64 dan x86_64), iOS, Android (arm64-v8a dan armeabi-v7a), dan browser (WebAssembly).
  Di ponsel, kode yang sama mendapatkan sentuhan, safe area, keyboard layar, dan scrolling
  dengan inersia.
- **Penggambaran ulang parsial.** Frame di-diff dan hanya area yang rusak yang digambar
  ulang, dengan garis tipis diselaraskan ke piksel fisik dengan cara yang sama di setiap
  backend.
- **Pustaka elemen**: tombol, tombol ikon dan 25 ikon, checkbox, radio button, switch,
  slider, select, tab, kolom teks dengan seleksi dan clipboard, progress bar, card, list
  tile, badge, avatar, tooltip, menu konteks, tautan, dan gambar. Elemen-elemen itu sendiri
  adalah template JSON (`lib/*.json`), dan komponen Anda sendiri ditulis dengan cara yang
  sama.
- **Layout ala Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, scrolling, list
  tervirtualisasi untuk puluhan ribu baris, dan `Flow` — paragraf berisi potongan teks inline
  dengan pembungkusan baris melintasi potongan-potongan itu.
- **Binding dan state**: path ke data (`@user.name`), interpolasi teks, state lokal sebuah
  komponen (`@~open`), selektor berdasarkan state (hover, pressed, focus), `switch`, dan
  beberapa fungsi (`eq`, `not`, `len`, `ratio`…). Struct dipetakan ke data dan sebaliknya:
  `#[derive(Data)]` di Rust, tabel field di C, refleksi tanpa makro di C++20.
- **Tema**: terang, gelap, dan "ikuti sistem", dibangun dari token yang bisa Anda timpa.
- **Gambar sendiri**: node `Canvas` memanggil fungsi Anda untuk menulis primitif ke dalam
  persegi panjangnya, berdampingan dengan elemen biasa.
- **Thread**: thread pekerja mengirim hasil kembali ke thread UI (`post`), di semua platform
  kecuali browser.
- **Live reload** pada build debug: dengan `UISON_RELOAD=folder`, file JSON layar dibaca
  ulang saat Anda menyimpannya.
- **Pengujian tanpa jendela**: sebuah skenario (`UISON_SCRIPT=scenario.json`) mengetuk,
  mengetik, menggulir, dan mengambil screenshot berdasarkan `key` elemen; skenario yang sama
  berjalan di semua platform, di Chrome headless, di simulator iOS, dan di ponsel.
- **API C dan C++**: `uison.h` (C99), `uison_kit.h` (helper untuk C), `uison.hpp` (C++17,
  dengan kemudahan tambahan di C++20), dan `uison_widgets.hpp` (widget C++20 yang
  di-generate dari skema yang sama dengan DSL Rust); pustaka statis dan dinamis, paket CMake,
  dan `pkg-config`. C dan C++ juga bisa di-build untuk browser dengan Emscripten:
  `em++ main.cpp libuison.a -o index.html`.

## Platform

| platform | backend | diuji pada |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO melalui runtime Objective-C | Mac Apple Silicon — mesin pengembangan utama |
| iOS | UIKit, CADisplayLink, kode CoreGraphics/CoreText yang sama dengan macOS | simulator dan iPhone 12 mini |
| Android | JNI yang ditulis tangan, `android.graphics.Canvas`, lapisan Java ~300 baris; APK di-build tanpa Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite, dan WIC, COM ditulis tangan; DPI per monitor, title bar gelap | PC Windows 11 pada 100/125/150 % dan dua monitor dengan skala berbeda |
| Linux | kode protokol Wayland sendiri dan X11 (xcb), Cairo + Pango, xkbcommon — semuanya lewat `dlopen`, sehingga binary hanya bergantung pada libc | Mutter, KWin, Sway, Weston, dan X11 headless (arm64 dan x86_64); Ubuntu 24.04 dengan GNOME di mesin virtual |
| Browser | `wasm32-unknown-unknown` tanpa wasm-bindgen, canvas 2D; C/C++ lewat Emscripten | Chrome (dan Chrome headless) |

Belum diuji: layar sentuh sungguhan di Windows, Windows on ARM (bisa di-build tetapi belum
dijalankan), Linux di perangkat keras sungguhan dan KDE, input mouse dan keyboard di GNOME,
Safari, dan Firefox.

## Ukuran dan kecepatan

Build release (`opt-level = "s"`, LTO, `panic = "abort"`, di-strip). Ukuran diambil dari
pengukuran proyek ini sendiri seiring perkembangannya, jadi build terbaru bisa berbeda
beberapa persen.

| program | ukuran |
|---|---|
| pelajaran 1 (penghitung di atas), Rust, macOS | ~0,7 MB |
| hal yang sama dalam C / C++ dengan `libuison.a`, macOS | 672 KB / 816 KB |
| Studio — demo utama (email dengan 2.000 pesan, kanban, kontak, grafik, formulir), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, tanpa perlu Visual C++ Redistributable |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, browser (`.wasm`, gzip) | ~220 KB |

| pengukuran | hasil |
|---|---|
| event → jendela tergambar, macOS | ~0,8 ms, dengan ~0,2 ms di antaranya adalah kerja uison sendiri |
| sentuhan → frame, iPhone 12 mini | 3,6 ms; scrolling list pada 60 fps yang stabil |
| penggambaran ulang penuh Studio, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| event mouse → frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| event → frame, Chrome | median 2,2 ms |
| peluncuran → frame pertama | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, browser 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), port dari sebuah program egui, memberikan
perbandingan langsung: di macOS arm64 binary-nya 857 KB alih-alih 5,1 MB, tanpa dependensi
alih-alih 362 crate di `Cargo.lock`, dengan kecepatan pemindaian dan penggunaan memori yang
sama.

## Dokumentasi

Dokumentasi tersedia dalam bahasa Inggris, dengan [versi bahasa Rusia](../ru/README.md) untuk
setiap halaman.

- [Pilih bahasa](../README.md) — tempat untuk memulai.
- Seri pelajaran, masing-masing bisa dibaca sendiri, mulai dari menyiapkan sistem hingga
  thread dan frame Anda sendiri: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Pelajaran inti adalah program yang sama dalam ketiga bahasa, dan screenshot-nya identik
  hingga ke piksel.
- [Bahasa layar](../screens/README.md): [binding dan nilai](../screens/bindings.md),
  [layout](../screens/layout.md), [elemen](../screens/elements.md),
  [tema](../screens/themes.md).
- [iOS dan Android](../mobile.md), [browser (WebAssembly)](../wasm.md).

## Contoh

| contoh | yang ditunjukkan |
|---|---|
| `studio` | demo utama: email dengan 2.000 pesan, kanban, kontak, grafik beranimasi, formulir, pengaturan; layout ponsel di bawah 700 pt |
| `gallery` | semua elemen pustaka |
| `disk-usage` | port dari aplikasi egui: diagram sunburst yang digambar di node `Canvas`, panel dan menu sebagai layar uison, pemindai paralel di atas `std` |
| `browser` | browser web kecil: parser HTML dan CSS serta cascade buatan sendiri, HTTP, dan HTTPS lewat TLS sistem; sebuah halaman menjadi layar uison |
| `todo` | daftar tugas dengan DSL Rust |
| `list10k` | list tervirtualisasi berisi 10.000 baris |
| `form2`, `settings` | layar formulir dan pengaturan |
| `demo`, `scroll`, `uibench` | primitif yang ditulis tangan, scrolling, benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Contoh C dan C++ ada di [`examples-c`](../../examples-c) dan [`examples-cpp`](../../examples-cpp),
masing-masing berupa proyek CMake tersendiri.

## Build dan pengujian

Anda memerlukan Rust stable; tidak ada yang lain di macOS, Windows (dengan toolchain MSVC),
dan Linux (tanpa paket `-dev`: pustaka sistem dimuat saat runtime). Menyiapkan setiap sistem
dibahas di pelajaran 0 setiap seri ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Di Windows skripnya adalah `tools\*.ps1`: `tools\win.ps1 studio` melakukan build, menjalankan
skenario, dan menyimpan screenshot; `tools\c-dist.ps1 -Test` mem-build dan menguji paket C/C++.

Sakelar backend untuk debugging, sama di mana pun: `UISON_STATS=1` mencetak waktu event,
frame, dan animasi, `UISON_FLASH=1` menyorot area yang digambar ulang, dan
`UISON_SCRIPT` / `UISON_CAPTURE` menjalankan skenario dan menyimpan screenshot.

## Struktur

```
crates/
  uison-json      parser JSON pull dan pohon nilai, dengan komentar, koma di akhir, dan key tanpa tanda kutip
  uison-core      geometri, warna, font, event, trait Canvas dan Host, trait App
  uison-frame     frame primitif: parsing, diff → area rusak, hit testing, eksekutor skenario
  uison-ui        kompiler layar: template, binding, tema, layout, runtime, DSL Rust, lib/*.json
  uison-apple     dipakai bersama macOS dan iOS: FFI Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     backend AppKit
  uison-ios       backend UIKit
  uison-android   backend JNI dan lapisan Java-nya (java/dev/uison)
  uison-windows   backend Win32 + Direct2D + DirectWrite
  uison-linux     backend Wayland dan X11, Cairo + Pango, semuanya lewat dlopen
  uison-web       backend WebAssembly (crates/uison-web/uison.js)
  uison-c         API C dan header C++, pustaka statis dan dinamis, paket CMake
  uison-gen       men-generate widget C++, DSL Rust, dan referensi elemen dari skema layar
  uison-derive    #[derive(Data)] tanpa syn
  uison           crate payung: semua untuk platform saat ini, uison::prelude; pelajaran Rust
examples/         contoh Rust (lihat di atas)
examples-c/       contoh dan pelajaran C, proyek CMake
examples-cpp/     contoh dan pelajaran C++, proyek CMake
scenarios/        skenario pengujian yang dipakai bersama semua platform
tools/            skrip build, jalankan, dan uji untuk setiap platform; driver input mouse dan keyboard sungguhan
web/              halaman dan sisi JavaScript dari backend WebAssembly
docs/             dokumentasi dalam bahasa Inggris; ru/, isi yang sama dalam bahasa Rusia; readme/, README ini dalam bahasa lain
```

## Keterbatasan

- Satu jendela per program; belum ada drag-and-drop.
- Input teks mencakup huruf Latin dan Kiril serta keyboard layar ponsel; input komposisi
  untuk bahasa Tionghoa, Jepang, dan Korea (IME) di desktop belum dibuat, begitu pula teks
  kanan-ke-kiri.
- Belum ada dukungan aksesibilitas (pembaca layar).
- Tidak ada thread di browser.
- Framework ini masih muda: versi 0.1, API bisa berubah.

## Terjemahan

Dokumentasi tersedia dalam bahasa Inggris, dan `docs/ru/` berisi halaman yang sama dalam
bahasa Rusia, dengan pengalih bahasa di bagian atas setiap halaman. README ini juga
diterjemahkan: `docs/readme/README.<code>.md`, satu per bahasa, dengan daftar bahasa di bagian
atas masing-masing. Perubahan di sini juga perlu diterapkan ke terjemahannya.

## Lisensi

Lisensi belum dipilih.
