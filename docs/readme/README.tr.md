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
<a href="README.te.md">🇮🇳 తెలుగు</a> ·
<b>🇹🇷 Türkçe</b> ·
<a href="README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON), Rust ile yazılmış, hiçbir bağımlılığı olmayan küçük ve hızlı bir UI
framework'üdür: yalnızca standart kütüphane kullanır, JSON ayrıştırıcısı bile kendisinindir.
Bir ekranı öğelerden oluşan bir ağaç olarak tanımlarsınız: bir JSON dosyasında, Rust DSL ile,
Flutter tarzı C++ widget'larıyla ya da doğrudan C kodunun içinde JSON olarak. Mantık Rust, C
veya C++ ile yazılmış sıradan koddur: handler'lar, veriler, thread'ler. Pencereyi, yerleşimi
(layout), çizimi ve girdiyi uison üstlenir; aynı program macOS, Windows, Linux (Wayland ve
X11), iOS, Android ve tarayıcıda çalışır.

Her backend sistemle elle yazılmış FFI üzerinden doğrudan konuşur: AppKit ve CoreText,
Direct2D ve DirectWrite ile Win32 (COM elle), sıfırdan uygulanmış Wayland protokolü ve X11,
UIKit, JNI üzerinden Android Canvas ve WebAssembly'den 2D canvas. Release derlemesi olan bir
program yaklaşık 0,7–0,9 MB boyutunda tek bir dosyadır, sistemin kendi kütüphanelerinden
başka hiçbir şeye ihtiyaç duymaz ve masaüstünde bir olay ekrana yaklaşık bir milisaniyede
ulaşır.

![uison ile yapılmış programlar: Studio (e-posta, ayarlar, grafikler), öğe galerisi, Todo, Disk Usage ve küçük bir web tarayıcısı](../img/showcase.jpg)

## Nasıl çalışır

```text
ekran (JSON, Rust DSL, C++ widget'ları, C'de UISON_JSON)  +  veri  +  kendi dilinizde handler'lar
        │  bağlamalar, şablonlar, temalar, yerleşim (uison-ui)
        ▼
primitiflerden oluşan frame: rect, line, circle, tri, poly, text, image, clip, hit
        │  önceki frame ile diff → yalnızca değişeni yeniden çiz
        ▼
sistem backend'i: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Bir ekran bir *frame*'e derlenir: birkaç tür primitiften oluşan düz bir liste —
dikdörtgenler, çizgiler, daireler, çokgenler, tek satırlık metinler, görseller, kırpma
bölgeleri ve fare ile dokunma için görünmez tıklama alanları. Her yeni frame bir öncekiyle
karşılaştırılır ve backend yalnızca değişen bölgeleri yeniden boyar. Bir backend küçüktür,
çünkü yalnızca bu primitifleri çizmesi, metni ölçmesi ve sistemin olaylarını uison'un
olaylarına dönüştürmesi gerekir.

Ekran dilinde kod yoktur. Metinler ve özellikler veriye bağlanır
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), öğeler parametreli, yeniden
kullanılabilir şablonlardır, renkler ve boyutlar tema token'larından gelir, olaylar ise
handler'ları Rust, C veya C++ ile yazılmış adlandırılmış eylemleri çağırır. Bir handler
veriyi değiştirdiğinde ekran kendini yeniden oluşturur.

## Merhaba, dünya

Aynı pencere — bir sayaç ve bir "+1" düğmesi — üç dilde.

**Rust** — ekran DSL ile kurulur, handler düğmedeki bir closure'dır:

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

**C** — ekran doğrudan koda yazılmış JSON'dur, handler bir fonksiyondur:

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

**C++** — Flutter'daki gibi widget'lar, handler düğmedeki bir lambda'dır:

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

Kendi projenize başlamak için (İngilizce) hızlı başlangıç rehberine bakın:
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) veya [C++](../cpp/quickstart.md).

## Özellikler

- **Bağımlılık yok.** Çekirdek, JSON ayrıştırıcısı, yerleşim, Wayland istemcisi, Linux ayar
  portalı için D-Bus istemcisi ve Windows için COM bağlamalarının hepsi burada, `std` üzerine
  yazılmıştır. `Cargo.lock` yalnızca workspace'in kendi crate'lerini içerir.
- **Tek program, altı platform.** macOS, Windows (x64 ve ARM64), Linux (Wayland ve X11,
  arm64 ve x86_64), iOS, Android (arm64-v8a ve armeabi-v7a) ve tarayıcı (WebAssembly).
  Telefonda aynı kod dokunma desteği, güvenli alan (safe area), ekran klavyesi ve ataletli
  kaydırma kazanır.
- **Kısmi yeniden çizim.** Frame'ler arasında diff alınır ve yalnızca hasar gören bölgeler
  yeniden boyanır; ince çizgiler her backend'de aynı şekilde fiziksel piksellere hizalanır.
- **Bir öğe kütüphanesi**: düğmeler, simge düğmeleri ve 25 simge, onay kutuları, radyo
  düğmeleri, anahtarlar (switch), kaydırıcılar (slider), açılır seçimler (select), sekmeler,
  seçim ve pano destekli metin alanları, ilerleme çubukları, kartlar, liste öğeleri,
  rozetler, avatarlar, tooltip'ler, bağlam menüleri, bağlantılar ve görseller. Öğelerin
  kendisi JSON şablonlarıdır (`lib/*.json`) ve kendi bileşenleriniz de aynı şekilde yazılır.
- **Flutter tarzı yerleşim**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex, kaydırma,
  on binlerce satır için sanallaştırılmış listeler ve `Flow` — aralarında satır kaydırma
  yapılan satır içi metin parçalarından oluşan bir paragraf.
- **Bağlamalar ve durum**: veri içindeki yollar (`@user.name`), metin enterpolasyonu, bir
  bileşenin yerel durumu (`@~open`), duruma göre seçiciler (hover, pressed, focus), `switch`
  ve birkaç fonksiyon (`eq`, `not`, `len`, `ratio`…). Struct'lar veriye ve geri dönüştürülür:
  Rust'ta `#[derive(Data)]`, C'de alan tabloları, C++20'de makrosuz yansıma (reflection).
- **Temalar**: açık, koyu ve "sistemi izle"; geçersiz kılabileceğiniz token'lardan oluşur.
- **Kendi çiziminiz**: bir `Canvas` düğümü, sıradan öğelerin yanında kendi dikdörtgenine
  primitif yazması için fonksiyonunuzu çağırır.
- **Thread'ler**: bir worker thread sonuçları UI thread'ine geri gönderir (`post`); tarayıcı
  dışındaki tüm platformlarda.
- Debug derlemelerinde **canlı yeniden yükleme**: `UISON_RELOAD=folder` ile ekranın JSON
  dosyaları kaydettiğiniz anda yeniden okunur.
- **Pencere olmadan test**: bir senaryo (`UISON_SCRIPT=scenario.json`) öğelerin `key`
  değerine göre dokunur, yazar, kaydırır ve ekran görüntüsü alır; aynı senaryolar her
  platformda, headless Chrome'da, iOS simülatöründe ve telefonda çalışır.
- **[Ajan modu](../agents.md)**: bir LLM ajanı programı ekran görüntüsü olmadan kullanır. Ekranı
  anlamıyla okur (roller, adlar, durumlar, anahtarlar) ve bir insanın girdisiyle aynı yoldan tıklar,
  yazar ve seçer; JSON satırlarıyla (`--agent`) ya da MCP sunucusu olarak (`--mcp`).
- **C ve C++ API**: `uison.h` (C99), `uison_kit.h` (C için yardımcılar), `uison.hpp` (C++17,
  C++20'de ek kolaylıklar) ve `uison_widgets.hpp` (Rust DSL ile aynı şemadan üretilen C++20
  widget'ları); statik ve dinamik kütüphaneler, bir CMake paketi ve `pkg-config`. C ve C++
  Emscripten ile tarayıcı için de derlenir: `em++ main.cpp libuison.a -o index.html`.

## Platformlar

| platform | backend | test edildiği yer |
|---|---|---|
| macOS | Objective-C runtime üzerinden AppKit, CoreGraphics, CoreText, ImageIO | Apple Silicon Mac — ana geliştirme makinesi |
| iOS | UIKit, CADisplayLink, macOS ile aynı CoreGraphics/CoreText kodu | simülatör ve iPhone 12 mini |
| Android | elle JNI, `android.graphics.Canvas`, ~300 satırlık bir Java katmanı; APK Gradle olmadan derlenir | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite ve WIC, elle COM; monitör başına DPI, koyu başlık çubuğu | %100/125/150 ölçekte Windows 11 PC ve farklı ölçekli iki monitör |
| Linux | kendi Wayland protokol kodu ve X11 (xcb), Cairo + Pango, xkbcommon — hepsi `dlopen` ile, bu yüzden binary yalnızca libc'ye bağımlıdır | headless Mutter, KWin, Sway, Weston ve X11 (arm64 ve x86_64); sanal makinede GNOME ile Ubuntu 24.04 |
| Tarayıcı | wasm-bindgen olmadan `wasm32-unknown-unknown`, canvas 2D; C/C++ için Emscripten | Chrome (ve headless Chrome) |

Henüz test edilmedi: Windows'ta gerçek bir dokunmatik ekran, Windows on ARM (derleniyor ama
çalıştırılmadı), gerçek donanımda Linux ve KDE, GNOME'da fare ve klavye girdisi, Safari ve
Firefox.

## Boyut ve hız

Release derlemeleri (`opt-level = "s"`, LTO, `panic = "abort"`, strip edilmiş). Boyutlar
projenin büyürken yaptığı kendi ölçümlerinden alınmıştır; en yeni derlemeler yüzde birkaç
farklı olabilir.

| program | boyut |
|---|---|
| ders 1 (yukarıdaki sayaç), Rust, macOS | ~0,7 MB |
| aynısı `libuison.a` ile C / C++'ta, macOS | 672 KB / 816 KB |
| Studio — ana demo (2.000 mesajlı e-posta, kanban, kişiler, grafikler, formlar), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, Visual C++ Redistributable gerekmez |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, tarayıcı (`.wasm`, gzip) | ~220 KB |

| ölçüm | sonuç |
|---|---|
| olay → boyanmış pencere, macOS | ~0,8 ms, bunun ~0,2 ms'si uison'un kendi işi |
| dokunma → frame, iPhone 12 mini | 3,6 ms; bir listeyi sabit 60 fps ile kaydırma |
| Studio'nun tamamen yeniden boyanması, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| fare olayı → frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| olay → frame, Chrome | medyan 2,2 ms |
| başlatma → ilk frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, tarayıcı 24–42 ms |

Bir egui programının portu olan [Disk Usage](../../examples/disk-usage/README.md) doğrudan
bir karşılaştırma sunar: macOS arm64'te binary 5,1 MB yerine 857 KB'tır, `Cargo.lock`'taki
362 crate yerine hiç bağımlılığı yoktur ve tarama hızı ile bellek kullanımı aynıdır.

## Dokümantasyon

Dokümantasyon İngilizcedir ve her sayfanın bir [Rusça sürümü](../ru/README.md) vardır.

- [Bir dil seçin](../README.md) — nereden başlamalı.
- Her biri tek başına okunabilen ders serileri; sisteminizi hazırlamaktan thread'lere ve
  kendi frame'inize kadar: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Temel dersler üç dilde de aynı programdır ve ekran görüntüleri piksel piksel aynıdır.
- [Ekran dili](../screens/README.md): [bağlamalar ve değerler](../screens/bindings.md),
  [yerleşim](../screens/layout.md), [öğeler](../screens/elements.md),
  [temalar](../screens/themes.md).
- [iOS ve Android](../mobile.md), [tarayıcı (WebAssembly)](../wasm.md).

## Örnekler

| örnek | ne gösterir |
|---|---|
| `studio` | ana demo: 2.000 mesajlı e-posta, kanban, kişiler, animasyonlu grafikler, bir form, ayarlar; 700 pt altı için telefon yerleşimi |
| `gallery` | kütüphanedeki her öğe |
| `disk-usage` | bir egui uygulamasının portu: `Canvas` düğümünde çizilen sunburst grafiği, uison ekranı olarak paneller ve menüler, `std` üzerinde paralel tarayıcı |
| `browser` | küçük bir web tarayıcısı: kendi HTML ve CSS ayrıştırıcısı ve cascade'i, HTTP ve sistemin TLS'i üzerinden HTTPS; bir sayfa uison ekranına dönüşür |
| `todo` | Rust DSL ile yapılacaklar listesi |
| `list10k` | 10.000 satırlık sanallaştırılmış liste |
| `form2`, `settings` | form ve ayar ekranları |
| `demo`, `scroll`, `uibench` | elle primitifler, kaydırma, bir benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C ve C++ örnekleri [`examples-c`](../../examples-c) ve [`examples-cpp`](../../examples-cpp)
klasörlerindedir; her biri kendi başına bir CMake projesidir.

## Derleme ve test

Stable Rust gerekir; macOS, Windows (MSVC toolchain ile) ve Linux'ta başka hiçbir şey
gerekmez (`-dev` paketleri yok: sistem kütüphaneleri çalışma zamanında yüklenir). Her sistemin
hazırlanması, her serinin 0. dersidir ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

Windows'ta betikler `tools\*.ps1` şeklindedir: `tools\win.ps1 studio` derler, bir senaryo
çalıştırır ve ekran görüntülerini kaydeder; `tools\c-dist.ps1 -Test` C/C++ paketini derler
ve test eder.

Hata ayıklama için backend anahtarları, her yerde aynıdır: `UISON_STATS=1` olayların,
frame'lerin ve animasyonun sürelerini yazdırır, `UISON_FLASH=1` yeniden boyanan bölgeleri
vurgular, `UISON_SCRIPT` / `UISON_CAPTURE` ise bir senaryo çalıştırıp ekran görüntülerini
kaydeder.

## Yapı

```
crates/
  uison-json      JSON pull ayrıştırıcısı ve değer ağacı; yorumlar, sondaki virgüller ve tırnaksız anahtarlarla
  uison-core      geometri, renkler, fontlar, olaylar, Canvas ve Host trait'leri, App trait'i
  uison-frame     primitif frame: ayrıştırma, diff → hasar, hit testing, senaryo çalıştırıcı
  uison-ui        ekran derleyicisi: şablonlar, bağlamalar, temalar, yerleşim, runtime, Rust DSL, lib/*.json
  uison-apple     macOS ve iOS ortak: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit backend'i
  uison-ios       UIKit backend'i
  uison-android   JNI backend'i ve Java katmanı (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite backend'i
  uison-linux     Wayland ve X11 backend'i, Cairo + Pango, hepsi dlopen ile
  uison-web       WebAssembly backend'i (crates/uison-web/uison.js)
  uison-c         C API'si ve C++ başlıkları, statik ve dinamik kütüphane, CMake paketi
  uison-gen       ekran şemasından C++ widget'larını, Rust DSL'i ve öğe başvuru belgesini üretir
  uison-derive    syn olmadan #[derive(Data)]
  uison           şemsiye crate: mevcut platform için her şey, uison::prelude; Rust dersleri
examples/         Rust örnekleri (yukarıya bakın)
examples-c/       C örnekleri ve dersleri, bir CMake projesi
examples-cpp/     C++ örnekleri ve dersleri, bir CMake projesi
scenarios/        tüm platformların ortak test senaryoları
tools/            her platform için derleme, çalıştırma ve test betikleri; gerçek fare ve klavye girdisi sürücüleri
web/              sayfa ve WebAssembly backend'inin JavaScript tarafı
docs/             İngilizce dokümantasyon; ru/, aynısı Rusça; readme/, bu README'nin diğer dillerdeki hâli
```

## Sınırlamalar

- Program başına tek pencere; sürükle-bırak yok.
- Metin girişi Latin ve Kiril alfabesini ve telefonların ekran klavyelerini kapsar;
  masaüstünde Çince, Japonca ve Korece için bileşik giriş (IME) yapılmadı, sağdan sola
  metin de yok.
- Henüz erişilebilirlik desteği (ekran okuyucular) yok.
- Tarayıcıda thread yok.
- Framework genç: sürüm 0.1, API değişebilir.

## Çeviriler

Dokümantasyon İngilizcedir; `docs/ru/` aynı sayfaları Rusça olarak içerir ve her sayfanın
üstünde bir dil değiştirici vardır. Bu README de çevrilmiştir:
`docs/readme/README.<code>.md`, her dil için bir tane, her birinin üstünde dillerin listesi
vardır. Burada yapılan bir değişiklik çevirilere de işlenmelidir.

## Lisans

Lisans henüz seçilmedi.
