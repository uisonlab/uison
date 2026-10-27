# uison

<!-- languages -->
<h3 align="center">
<a href="../../README.md">🇬🇧 English</a> ·
<a href="README.zh.md">🇨🇳 中文</a> ·
<a href="README.hi.md">🇮🇳 हिन्दी</a> ·
<b>🇪🇸 Español</b> ·
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

**uison** (UI + JSON) es un framework de UI pequeño y rápido escrito en Rust, sin ninguna
dependencia: solo la biblioteca estándar, hasta el punto de tener su propio parser de JSON.
Describes una pantalla como un árbol de elementos: en un archivo JSON, con un DSL de Rust,
con widgets de C++ al estilo de Flutter o como JSON directamente dentro del código C. La
lógica es código normal en Rust, C o C++: handlers, datos, hilos. uison se encarga de la
ventana, el layout, el dibujo y la entrada, y el mismo programa funciona en macOS, Windows,
Linux (Wayland y X11), iOS, Android y en el navegador.

Cada backend habla directamente con el sistema mediante FFI escrito a mano: AppKit y
CoreText, Win32 con Direct2D y DirectWrite (COM a mano), el protocolo Wayland implementado
desde cero y X11, UIKit, el Canvas de Android a través de JNI y un canvas 2D desde
WebAssembly. Un programa en release es un único archivo de unos 0,7–0,9 MB que no necesita
nada más que las bibliotecas del propio sistema, y un evento llega a la pantalla en
aproximadamente un milisegundo en un equipo de escritorio.

![Disk Usage, un ejemplo hecho con uison: diagrama solar (sunburst) de una carpeta, lista de archivos y un tooltip](../img/disk-usage.jpg)

## Cómo funciona

```text
pantalla (JSON, DSL de Rust, widgets de C++, UISON_JSON en C)  +  datos  +  handlers en tu lenguaje
        │  bindings, plantillas, temas, layout (uison-ui)
        ▼
frame de primitivas: rect, line, circle, tri, poly, text, image, clip, hit
        │  diff con el frame anterior → redibujar solo lo que cambió
        ▼
backend del sistema: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Una pantalla se compila en un *frame*: una lista plana de unos pocos tipos de primitivas:
rectángulos, líneas, círculos, polígonos, líneas sueltas de texto, imágenes, regiones de
recorte y áreas de impacto invisibles para el ratón y el tacto. Cada frame nuevo se compara
con el anterior, y el backend vuelve a pintar solo las regiones que cambiaron. Un backend es
pequeño porque solo tiene que dibujar estas primitivas, medir texto y convertir los eventos
del sistema en los de uison.

El lenguaje de pantallas no contiene código. El texto y las propiedades se enlazan a los
datos (`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), los elementos son plantillas
reutilizables con parámetros, los colores y tamaños vienen de tokens del tema, y los eventos
invocan acciones con nombre cuyos handlers se escriben en Rust, C o C++. Cuando un handler
cambia los datos, la pantalla se reconstruye sola.

## Hola, mundo

La misma ventana —un contador y un botón «+1»— en los tres lenguajes.

**Rust**: la pantalla se construye con un DSL y el handler es un closure en el botón:

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

**C**: la pantalla es JSON escrito directamente en el código y el handler es una función:

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

**C++**: widgets como en Flutter y el handler es una lambda en el botón:

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

Para empezar tu propio proyecto, consulta la guía rápida (en inglés) para
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) o [C++](../cpp/quickstart.md).

## Características

- **Sin dependencias.** El núcleo, el parser de JSON, el layout, el cliente de Wayland, el
  cliente de D-Bus para el portal de configuración de Linux y los bindings COM para Windows
  están escritos aquí, sobre `std`. `Cargo.lock` solo contiene los crates del propio workspace.
- **Un programa, seis plataformas.** macOS, Windows (x64 y ARM64), Linux (Wayland y X11,
  arm64 y x86_64), iOS, Android (arm64-v8a y armeabi-v7a) y el navegador (WebAssembly). En un
  teléfono, el mismo código obtiene entrada táctil, área segura, teclado en pantalla y
  desplazamiento con inercia.
- **Redibujado parcial.** Se calcula el diff entre frames y solo se repintan las regiones
  dañadas; las líneas finas se ajustan a píxeles físicos de la misma forma en todos los
  backends.
- **Una biblioteca de elementos**: botones, botones de icono y 25 iconos, casillas de
  verificación, botones de opción, interruptores, sliders, selects, pestañas, campos de texto
  con selección y portapapeles, barras de progreso, tarjetas, elementos de lista, badges,
  avatares, tooltips, menús contextuales, enlaces e imágenes. Los propios elementos son
  plantillas JSON (`lib/*.json`), y tus componentes se escriben de la misma manera.
- **Layout al estilo de Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex,
  desplazamiento, listas virtualizadas para decenas de miles de filas y `Flow`: un párrafo de
  fragmentos de texto en línea con ajuste de línea entre ellos.
- **Bindings y estado**: rutas hacia los datos (`@user.name`), interpolación de texto, estado
  local de un componente (`@~open`), selectores por estado (hover, pressed, focus), `switch` y
  unas pocas funciones (`eq`, `not`, `len`, `ratio`…). Las structs se convierten en datos y
  viceversa: `#[derive(Data)]` en Rust, tablas de campos en C, reflexión sin macros en C++20.
- **Temas**: claro, oscuro y «seguir al sistema», construidos con tokens que puedes
  sobrescribir.
- **Dibujo propio**: un nodo `Canvas` llama a tu función para que escriba primitivas en su
  rectángulo, junto a los elementos normales.
- **Hilos**: un hilo de trabajo envía los resultados de vuelta al hilo de la UI (`post`), en
  todas las plataformas excepto el navegador.
- **Recarga en vivo** en builds de depuración: con `UISON_RELOAD=folder`, los archivos JSON
  de la pantalla se vuelven a leer al guardarlos.
- **Pruebas sin ventana**: un escenario (`UISON_SCRIPT=scenario.json`) toca, escribe, desplaza
  y hace capturas de pantalla según la `key` de los elementos; los mismos escenarios se
  ejecutan en todas las plataformas, en Chrome headless, en el simulador de iOS y en un
  teléfono.
- **API de C y C++**: `uison.h` (C99), `uison_kit.h` (utilidades para C), `uison.hpp` (C++17,
  con comodidades en C++20) y `uison_widgets.hpp` (widgets de C++20 generados a partir del
  mismo esquema que el DSL de Rust); bibliotecas estáticas y dinámicas, un paquete CMake y
  `pkg-config`. C y C++ también compilan para el navegador con Emscripten:
  `em++ main.cpp libuison.a -o index.html`.

## Plataformas

| plataforma | backend | probado en |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO a través del runtime de Objective-C | Mac con Apple Silicon, la máquina principal de desarrollo |
| iOS | UIKit, CADisplayLink, el mismo código CoreGraphics/CoreText que en macOS | simulador e iPhone 12 mini |
| Android | JNI a mano, `android.graphics.Canvas`, una capa Java de ~300 líneas; APK compilado sin Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite y WIC, COM a mano; DPI por monitor, barra de título oscura | PC con Windows 11 al 100/125/150 % y dos monitores con escalas distintas |
| Linux | código propio del protocolo Wayland y X11 (xcb), Cairo + Pango, xkbcommon, todo mediante `dlopen`, por lo que el binario solo depende de libc | Mutter, KWin, Sway, Weston y X11 headless (arm64 y x86_64); Ubuntu 24.04 con GNOME en una máquina virtual |
| Navegador | `wasm32-unknown-unknown` sin wasm-bindgen, canvas 2D; C/C++ mediante Emscripten | Chrome (y Chrome headless) |

Aún sin probar: una pantalla táctil real en Windows, Windows on ARM (compila, pero no se ha
ejecutado), Linux en hardware real y KDE, la entrada de ratón y teclado en GNOME, Safari y
Firefox.

## Tamaño y velocidad

Builds de release (`opt-level = "s"`, LTO, `panic = "abort"`, sin símbolos). Los tamaños
provienen de las mediciones del propio proyecto a medida que crecía, así que los builds más
recientes pueden diferir en un pequeño porcentaje.

| programa | tamaño |
|---|---|
| lección 1 (el contador de arriba), Rust, macOS | ~0,7 MB |
| lo mismo en C / C++ con `libuison.a`, macOS | 672 KB / 816 KB |
| Studio, la demo principal (correo con 2000 mensajes, kanban, contactos, gráficos, formularios), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, sin necesidad de Visual C++ Redistributable |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, navegador (`.wasm`, gzip) | ~220 KB |

| medición | resultado |
|---|---|
| evento → ventana pintada, macOS | ~0,8 ms, de los cuales ~0,2 ms son trabajo del propio uison |
| toque → frame, iPhone 12 mini | 3,6 ms; desplazamiento de una lista a 60 fps constantes |
| repintado completo de Studio, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| evento de ratón → frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| evento → frame, Chrome | mediana 2,2 ms |
| arranque → primer frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, navegador 24–42 ms |

[Disk Usage](../../examples/disk-usage/README.md), un port de un programa hecho con egui,
permite una comparación directa: en macOS arm64 el binario ocupa 857 KB en lugar de 5,1 MB,
sin dependencias en lugar de 362 crates en `Cargo.lock`, y con la misma velocidad de
escaneo y el mismo uso de memoria.

## Documentación

La documentación está en inglés, con una [versión en ruso](../ru/README.md) de cada página.

- [Elige un lenguaje](../README.md): por dónde empezar.
- Series de lecciones, cada una legible por separado, desde la preparación del sistema hasta
  los hilos y tu propio frame: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  Las lecciones básicas son el mismo programa en los tres lenguajes, y sus capturas coinciden
  píxel a píxel.
- [El lenguaje de pantallas](../screens/README.md): [bindings y valores](../screens/bindings.md),
  [layout](../screens/layout.md), [elementos](../screens/elements.md),
  [temas](../screens/themes.md).
- [iOS y Android](../mobile.md), [el navegador (WebAssembly)](../wasm.md).

## Ejemplos

| ejemplo | qué muestra |
|---|---|
| `studio` | la demo principal: correo con 2000 mensajes, kanban, contactos, gráficos animados, un formulario, ajustes; un layout para teléfono por debajo de 700 pt |
| `gallery` | todos los elementos de la biblioteca |
| `disk-usage` | un port de una app hecha con egui: un diagrama solar dibujado en un nodo `Canvas`, paneles y menús como pantalla de uison, un escáner paralelo sobre `std` |
| `browser` | un pequeño navegador web: su propio parser de HTML y CSS con cascada, HTTP y HTTPS mediante el TLS del sistema; una página se convierte en una pantalla de uison |
| `todo` | una lista de tareas con el DSL de Rust |
| `list10k` | una lista virtualizada de 10 000 filas |
| `form2`, `settings` | pantallas de formularios y de ajustes |
| `demo`, `scroll`, `uibench` | primitivas a mano, desplazamiento, un benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Los ejemplos de C y C++ están en [`examples-c`](../../examples-c) y [`examples-cpp`](../../examples-cpp),
cada uno un proyecto CMake independiente.

## Compilación y pruebas

Necesitas Rust stable; nada más en macOS, Windows (con el toolchain MSVC) y Linux (sin
paquetes `-dev`: las bibliotecas del sistema se cargan en tiempo de ejecución). La
preparación de cada sistema es la lección 0 de cada serie ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

En Windows los scripts son `tools\*.ps1`: `tools\win.ps1 studio` compila, ejecuta un
escenario y guarda capturas; `tools\c-dist.ps1 -Test` compila y prueba el paquete de C/C++.

Interruptores del backend para depuración, iguales en todas partes: `UISON_STATS=1` imprime
los tiempos de eventos, frames y animación, `UISON_FLASH=1` resalta las regiones repintadas,
y `UISON_SCRIPT` / `UISON_CAPTURE` ejecutan un escenario y guardan capturas de pantalla.

## Estructura

```
crates/
  uison-json      parser JSON pull y árbol de valores, con comentarios, comas finales y claves sin comillas
  uison-core      geometría, colores, fuentes, eventos, los traits Canvas y Host, el trait App
  uison-frame     el frame de primitivas: parsing, diff → daño, hit testing, el ejecutor de escenarios
  uison-ui        el compilador de pantallas: plantillas, bindings, temas, layout, el runtime, el DSL de Rust, lib/*.json
  uison-apple     común a macOS e iOS: FFI de Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     backend AppKit
  uison-ios       backend UIKit
  uison-android   backend JNI y su capa Java (java/dev/uison)
  uison-windows   backend Win32 + Direct2D + DirectWrite
  uison-linux     backend Wayland y X11, Cairo + Pango, todo mediante dlopen
  uison-web       backend WebAssembly (web/uison.js)
  uison-c         la API de C y las cabeceras de C++, biblioteca estática y dinámica, paquete CMake
  uison-gen       genera los widgets de C++, el DSL de Rust y la referencia de elementos a partir del esquema de pantallas
  uison-derive    #[derive(Data)] sin syn
  uison           el crate paraguas: todo para la plataforma actual, uison::prelude; las lecciones de Rust
examples/         ejemplos en Rust (ver arriba)
examples-c/       ejemplos y lecciones en C, un proyecto CMake
examples-cpp/     ejemplos y lecciones en C++, un proyecto CMake
scenarios/        escenarios de prueba comunes a todas las plataformas
tools/            scripts de compilación, ejecución y pruebas para cada plataforma; drivers de entrada real de ratón y teclado
web/              la página y la parte JavaScript del backend WebAssembly
docs/             documentación en inglés; ru/, la misma en ruso; readme/, este README en otros idiomas
```

## Limitaciones

- Una ventana por programa; sin arrastrar y soltar.
- La entrada de texto cubre el alfabeto latino y el cirílico, y los teclados en pantalla de
  los teléfonos; la entrada compuesta para chino, japonés y coreano (IME) en escritorio no
  está hecha, ni tampoco el texto de derecha a izquierda.
- Todavía no hay soporte de accesibilidad (lectores de pantalla).
- No hay hilos en el navegador.
- El framework es joven: versión 0.1, la API puede cambiar.

## Traducciones

La documentación está en inglés, y `docs/ru/` contiene las mismas páginas en ruso, con un
selector de idioma al principio de cada una. Este README también está traducido:
`docs/readme/README.<code>.md`, uno por idioma, con la lista de idiomas al principio de
cada uno. Un cambio aquí debe llevarse también a las traducciones.

## Licencia

La licencia aún no se ha elegido.
