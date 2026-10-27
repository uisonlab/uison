# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
<a href="README.es.md">🇪🇸 Español</a> ·
<b>🇫🇷 Français</b> ·
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

**uison** (UI + JSON) est un petit framework d'interface rapide écrit en Rust, sans aucune
dépendance : uniquement la bibliothèque standard, jusqu'à son propre parseur JSON. On décrit
un écran comme un arbre d'éléments : dans un fichier JSON, avec un DSL Rust, avec des widgets
C++ à la manière de Flutter, ou en JSON directement dans du code C. La logique est du code
ordinaire en Rust, C ou C++ : handlers, données, threads. uison s'occupe de la fenêtre, de la
mise en page, du dessin et des entrées, et le même programme tourne sous macOS, Windows,
Linux (Wayland et X11), iOS, Android et dans le navigateur.

Chaque backend parle directement au système, via un FFI écrit à la main : AppKit et CoreText,
Win32 avec Direct2D et DirectWrite (COM à la main), le protocole Wayland implémenté de zéro
et X11, UIKit, le Canvas d'Android via JNI et un canvas 2D depuis WebAssembly. Un programme
en release est un seul fichier d'environ 0,7–0,9 Mo qui n'a besoin que des bibliothèques du
système, et un événement atteint l'écran en environ une milliseconde sur un ordinateur de bureau.

![Programmes construits avec uison : Studio (courrier, réglages, graphiques), la galerie d'éléments, Todo, Disk Usage et un petit navigateur web](../img/showcase.jpg)

## Fonctionnement

```text
écran (JSON, DSL Rust, widgets C++, UISON_JSON en C)  +  données  +  handlers dans votre langage
        │  liaisons, modèles, thèmes, mise en page (uison-ui)
        ▼
frame de primitives : rect, line, circle, tri, poly, text, image, clip, hit
        │  diff avec la frame précédente → ne redessiner que ce qui a changé
        ▼
backend système : AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Un écran se compile en une *frame* : une liste plate de quelques sortes de primitives —
rectangles, lignes, cercles, polygones, textes d'une seule ligne, images, zones de découpage
et zones de clic invisibles pour la souris et le tactile. Chaque nouvelle frame est comparée
à la précédente, et le backend ne redessine que les zones qui ont changé. Un backend reste
petit, car il lui suffit de dessiner ces primitives, de mesurer le texte et de traduire les
événements du système en événements uison.

Le langage d'écrans ne contient pas de code. Les textes et les propriétés sont liés aux
données (`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), les éléments sont des modèles
réutilisables avec paramètres, les couleurs et les tailles viennent des tokens du thème, et
les événements appellent des actions nommées dont les handlers sont écrits en Rust, C ou C++.
Quand un handler modifie les données, l'écran se reconstruit tout seul.

## Hello, world

La même fenêtre — un compteur et un bouton « +1 » — dans les trois langages.

**Rust** — l'écran est construit avec un DSL, le handler est une closure sur le bouton :

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

**C** — l'écran est du JSON écrit directement dans le code, le handler est une fonction :

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

**C++** — des widgets comme dans Flutter, le handler est une lambda sur le bouton :

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

Pour démarrer votre propre projet, voir le démarrage rapide pour
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) ou [C++](../cpp/quickstart.md).

## Fonctionnalités

- **Aucune dépendance.** Le cœur, le parseur JSON, la mise en page, le client Wayland, le
  client D-Bus pour le portail de paramètres de Linux et les bindings COM pour Windows sont
  tous écrits ici, au-dessus de `std`. `Cargo.lock` ne liste que les crates du workspace.
- **Un programme, six plateformes.** macOS, Windows (x64 et ARM64), Linux (Wayland et X11,
  arm64 et x86_64), iOS, Android (arm64-v8a et armeabi-v7a) et le navigateur (WebAssembly).
  Sur un téléphone, le même code bénéficie du tactile, d'une zone de sécurité, du clavier
  virtuel et du défilement inertiel.
- **Redessin partiel.** Les frames sont comparées par diff et seules les zones endommagées
  sont redessinées ; les lignes fines sont alignées sur les pixels physiques de la même
  façon dans chaque backend.
- **Une bibliothèque d'éléments** : boutons, boutons-icônes et 25 icônes, cases à cocher,
  boutons radio, interrupteurs, curseurs, listes déroulantes, onglets, champs de texte avec
  sélection et presse-papiers, barres de progression, cartes, lignes de liste, badges,
  avatars, infobulles, menus contextuels, liens et images. Les éléments sont eux-mêmes des
  modèles JSON (`lib/*.json`), et vos propres composants s'écrivent de la même façon.
- **Une mise en page dans l'esprit de Flutter** : `Row`, `Column`, `Stack`, `Wrap`, `Align`,
  flex, défilement, listes virtualisées pour des dizaines de milliers de lignes, et `Flow` —
  un paragraphe de fragments de texte en ligne, avec retour à la ligne à travers eux.
- **Liaisons et état** : chemins dans les données (`@user.name`), interpolation de texte,
  état local d'un composant (`@~open`), sélecteurs selon l'état (hover, pressed, focus),
  `switch` et quelques fonctions (`eq`, `not`, `len`, `ratio`…). Les structures se
  convertissent en données et inversement : `#[derive(Data)]` en Rust, tables de champs en C,
  réflexion sans macros en C++20.
- **Thèmes** : clair, sombre et « comme le système », construits à partir de tokens
  personnalisables.
- **Votre propre dessin** : un nœud `Canvas` appelle votre fonction, qui écrit des primitives
  dans son rectangle, à côté des éléments ordinaires.
- **Threads** : un thread de travail renvoie ses résultats au thread de l'interface (`post`),
  sur toutes les plateformes sauf le navigateur.
- **Rechargement à chaud** en debug : avec `UISON_RELOAD=folder`, les fichiers JSON de
  l'écran sont relus à chaque enregistrement.
- **Tests sans fenêtre** : un scénario (`UISON_SCRIPT=scenario.json`) touche, saisit du texte,
  fait défiler et prend des captures d'écran d'après la `key` des éléments ; les mêmes
  scénarios tournent sur chaque plateforme, dans Chrome headless, dans le simulateur iOS et
  sur un téléphone.
- **API C et C++** : `uison.h` (C99), `uison_kit.h` (utilitaires pour C), `uison.hpp` (C++17,
  facilités en C++20) et `uison_widgets.hpp` (widgets C++20 générés à partir du même schéma
  que le DSL Rust) ; bibliothèques statique et dynamique, un paquet CMake et `pkg-config`.
  C et C++ se compilent aussi pour le navigateur avec Emscripten : `em++ main.cpp libuison.a -o index.html`.

## Plateformes

| plateforme | backend | vérifié sur |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO via le runtime Objective-C | Mac Apple Silicon — la machine de développement principale |
| iOS | UIKit, CADisplayLink, le même code CoreGraphics/CoreText que sous macOS | simulateur et iPhone 12 mini |
| Android | JNI à la main, `android.graphics.Canvas`, une couche Java d'environ 300 lignes ; APK construit sans Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite et WIC, COM à la main ; DPI par écran, barre de titre sombre | PC sous Windows 11 à 100/125/150 % et deux écrans à mises à l'échelle différentes |
| Linux | code maison pour le protocole Wayland et X11 (xcb), Cairo + Pango, xkbcommon — le tout via `dlopen`, si bien que le binaire ne dépend que de la libc | Mutter, KWin, Sway, Weston et X11 headless (arm64 et x86_64) ; Ubuntu 24.04 avec GNOME dans une machine virtuelle |
| Navigateur | `wasm32-unknown-unknown` sans wasm-bindgen, canvas 2D ; C/C++ via Emscripten | Chrome (et Chrome headless) |

Pas encore vérifié : un véritable écran tactile sous Windows, Windows on ARM (compile mais
n'a pas été lancé), Linux sur du vrai matériel et KDE, la saisie à la souris et au clavier
sous GNOME, Safari et Firefox.

## Taille et vitesse

Builds de release (`opt-level = "s"`, LTO, `panic = "abort"`, sans symboles). Les tailles
proviennent des mesures du projet au fil de son développement ; les builds les plus récents
peuvent donc différer de quelques pour cent.

| programme | taille |
|---|---|
| leçon 1 (le compteur ci-dessus), Rust, macOS | ~0,7 Mo |
| la même en C / C++ avec `libuison.a`, macOS | 672 Ko / 816 Ko |
| Studio — la démo principale (messagerie avec 2 000 messages, kanban, contacts, graphiques, formulaires), macOS | ~840 Ko |
| Studio, Windows x64 / ARM64 | 672 Ko / 613 Ko, sans Visual C++ Redistributable |
| Studio, Linux arm64 / x86_64 | 857 Ko / 936 Ko |
| Studio, Android `libmain.so` arm64 | ~820 Ko |
| Studio, navigateur (`.wasm`, gzip) | ~220 Ko |

| mesure | résultat |
|---|---|
| événement → fenêtre redessinée, macOS | ~0,8 ms, dont ~0,2 ms de travail propre à uison |
| toucher → frame, iPhone 12 mini | 3,6 ms ; défilement d'une liste à 60 i/s constants |
| redessin complet de Studio, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| événement souris → frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| événement → frame, Chrome | médiane 2,2 ms |
| lancement → première frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, navigateur 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), portage d'un programme egui, permet une
comparaison directe : sous macOS arm64, le binaire pèse 857 Ko au lieu de 5,1 Mo, sans
dépendances au lieu de 362 crates dans `Cargo.lock`, avec la même vitesse d'analyse et la
même consommation mémoire.

## Documentation

La documentation est en anglais, avec une [version russe](../ru/README.md) de chaque page.

- [Choisir un langage](../README.md) — par où commencer.
- Des séries de leçons, chacune lisible indépendamment, de la préparation du système aux
  threads et à votre propre frame : [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Les leçons de base sont le même programme dans les trois langages, et leurs captures
  d'écran sont identiques au pixel près.
- [Le langage d'écrans](../screens/README.md) : [liaisons et valeurs](../screens/bindings.md),
  [mise en page](../screens/layout.md), [éléments](../screens/elements.md),
  [thèmes](../screens/themes.md).
- [iOS et Android](../mobile.md), [le navigateur (WebAssembly)](../wasm.md).

## Exemples

| exemple | ce qu'il montre |
|---|---|
| `studio` | la démo principale : messagerie avec 2 000 messages, kanban, contacts, graphiques animés, un formulaire, des réglages ; une mise en page téléphone en dessous de 700 pt |
| `gallery` | tous les éléments de la bibliothèque |
| `disk-usage` | portage d'une application egui : un diagramme en rayons de soleil dessiné dans un nœud `Canvas`, panneaux et menus sous forme d'écran uison, un analyseur parallèle sur `std` |
| `browser` | un petit navigateur web : ses propres parseurs HTML et CSS et sa cascade, HTTP, et HTTPS via le TLS du système ; une page devient un écran uison |
| `todo` | une liste de tâches avec le DSL Rust |
| `list10k` | une liste virtualisée de 10 000 lignes |
| `form2`, `settings` | formulaires et écrans de réglages |
| `demo`, `scroll`, `uibench` | primitives à la main, défilement, un benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Les exemples C et C++ se trouvent dans [`examples-c`](../../examples-c) et [`examples-cpp`](../../examples-cpp),
chacun étant un projet CMake indépendant.

## Compilation et tests

Il vous faut Rust stable ; rien d'autre sous macOS, Windows (avec la toolchain MSVC) et Linux
(pas de paquets `-dev` : les bibliothèques système sont chargées à l'exécution). La
préparation de chaque système fait l'objet de la leçon 0 de chaque série
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

Sous Windows, les scripts sont `tools\*.ps1` : `tools\win.ps1 studio` compile, exécute un
scénario et enregistre des captures d'écran, `tools\c-dist.ps1 -Test` construit et teste le
paquet C/C++.

Options du backend pour le débogage, identiques partout : `UISON_STATS=1` affiche les temps
des événements, des frames et des animations, `UISON_FLASH=1` met en évidence les zones
redessinées, et `UISON_SCRIPT` / `UISON_CAPTURE` exécutent un scénario et enregistrent des
captures d'écran.

## Organisation

```
crates/
  uison-json      parseur JSON pull et arbre de valeurs, avec commentaires, virgules finales et clés sans guillemets
  uison-core      géométrie, couleurs, polices, événements, les traits Canvas et Host, le trait App
  uison-frame     la frame de primitives : analyse, diff → zones endommagées, hit testing, l'exécuteur de scénarios
  uison-ui        le compilateur d'écrans : modèles, liaisons, thèmes, mise en page, runtime, DSL Rust, lib/*.json
  uison-apple     commun à macOS et iOS : FFI Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     backend AppKit
  uison-ios       backend UIKit
  uison-android   backend JNI et sa couche Java (java/dev/uison)
  uison-windows   backend Win32 + Direct2D + DirectWrite
  uison-linux     backend Wayland et X11, Cairo + Pango, le tout via dlopen
  uison-web       backend WebAssembly (web/uison.js)
  uison-c         l'API C et les en-têtes C++, bibliothèque statique et dynamique, paquet CMake
  uison-gen       génère les widgets C++, le DSL Rust et la référence des éléments à partir du schéma d'écrans
  uison-derive    #[derive(Data)] sans syn
  uison           la crate parapluie : tout pour la plateforme courante, uison::prelude ; les leçons Rust
examples/         exemples Rust (voir ci-dessus)
examples-c/       exemples et leçons en C, un projet CMake
examples-cpp/     exemples et leçons en C++, un projet CMake
scenarios/        scénarios de test communs à toutes les plateformes
tools/            scripts de compilation, de lancement et de test pour chaque plateforme ; pilotes de vraies saisies souris et clavier
web/              la page et la partie JavaScript du backend WebAssembly
docs/             documentation en anglais ; ru/, la même en russe ; readme/, ce README dans d'autres langues
```

## Limites

- Une seule fenêtre par programme ; pas de glisser-déposer.
- La saisie de texte couvre l'alphabet latin et cyrillique et les claviers virtuels des
  téléphones ; la saisie composée pour le chinois, le japonais et le coréen (IME) sur
  ordinateur n'est pas faite, pas plus que le texte de droite à gauche.
- Pas encore d'accessibilité (lecteurs d'écran).
- Pas de threads dans le navigateur.
- Le framework est jeune : version 0.1, l'API peut changer.

## Traductions

La documentation est en anglais, et `docs/ru/` contient les mêmes pages en russe, avec un
sélecteur de langue en haut de chacune. Ce README est lui aussi traduit :
`docs/readme/README.<code>.md`, un par langue, avec la liste des langues en haut de chacun.
Toute modification ici doit aussi être reportée dans les traductions.

## Licence

La licence n'a pas encore été choisie.
