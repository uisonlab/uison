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
<b>🇧🇷 Português</b> ·
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

**uison** (UI + JSON) é um framework de UI pequeno e rápido escrito em Rust, sem nenhuma
dependência: só a biblioteca padrão, a ponto de ter o próprio parser de JSON. Você descreve
uma tela como uma árvore de elementos: em um arquivo JSON, com uma DSL em Rust, com widgets
C++ no estilo do Flutter ou como JSON direto dentro do código C. A lógica é código comum em
Rust, C ou C++: handlers, dados, threads. O uison cuida da janela, do layout, do desenho e
da entrada, e o mesmo programa roda no macOS, Windows, Linux (Wayland e X11), iOS, Android
e no navegador.

Cada backend conversa diretamente com o sistema, via FFI escrito à mão: AppKit e CoreText,
Win32 com Direct2D e DirectWrite (COM à mão), o protocolo Wayland implementado do zero e
X11, UIKit, o Canvas do Android via JNI e um canvas 2D a partir do WebAssembly. Um programa
em release é um único arquivo de cerca de 0,7–0,9 MB que não precisa de nada além das
bibliotecas do próprio sistema, e um evento chega à tela em cerca de um milissegundo em um
desktop.

![Disk Usage, um exemplo feito com uison: gráfico sunburst de uma pasta, lista de arquivos e um tooltip](../img/disk-usage.jpg)

## Como funciona

```text
tela (JSON, DSL em Rust, widgets C++, UISON_JSON em C)  +  dados  +  handlers na sua linguagem
        │  bindings, templates, temas, layout (uison-ui)
        ▼
frame de primitivas: rect, line, circle, tri, poly, text, image, clip, hit
        │  diff com o frame anterior → redesenhar só o que mudou
        ▼
backend do sistema: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

Uma tela é compilada em um *frame*: uma lista plana de poucos tipos de primitivas —
retângulos, linhas, círculos, polígonos, linhas únicas de texto, imagens, regiões de recorte
e áreas de clique invisíveis para mouse e toque. Cada novo frame é comparado com o anterior,
e o backend repinta só as regiões que mudaram. Um backend é pequeno porque só precisa
desenhar essas primitivas, medir texto e converter os eventos do sistema nos do uison.

A linguagem de telas não tem código. Textos e propriedades são vinculados aos dados
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), elementos são templates reutilizáveis
com parâmetros, cores e tamanhos vêm de tokens do tema, e eventos chamam ações nomeadas cujos
handlers são escritos em Rust, C ou C++. Quando um handler altera os dados, a tela se
reconstrói sozinha.

## Olá, mundo

A mesma janela — um contador e um botão "+1" — nas três linguagens.

**Rust** — a tela é montada com uma DSL, o handler é uma closure no botão:

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

**C** — a tela é JSON escrito direto no código, o handler é uma função:

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

**C++** — widgets como no Flutter, o handler é uma lambda no botão:

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

Para começar um projeto seu, veja o guia de início rápido (em inglês) para
[Rust](../rust/quickstart.md), [C](../c/quickstart.md) ou [C++](../cpp/quickstart.md).

## Recursos

- **Sem dependências.** O núcleo, o parser de JSON, o layout, o cliente Wayland, o cliente
  D-Bus para o portal de configurações do Linux e os bindings COM para Windows são todos
  escritos aqui, sobre `std`. O `Cargo.lock` lista só os crates do próprio workspace.
- **Um programa, seis plataformas.** macOS, Windows (x64 e ARM64), Linux (Wayland e X11,
  arm64 e x86_64), iOS, Android (arm64-v8a e armeabi-v7a) e o navegador (WebAssembly). No
  celular, o mesmo código ganha toque, safe area, teclado virtual e rolagem com inércia.
- **Redesenho parcial.** Os frames passam por diff e só as regiões afetadas são repintadas,
  com linhas finas alinhadas a pixels físicos do mesmo jeito em todos os backends.
- **Uma biblioteca de elementos**: botões, botões de ícone e 25 ícones, checkboxes, radio
  buttons, switches, sliders, selects, abas, campos de texto com seleção e área de
  transferência, barras de progresso, cards, itens de lista, badges, avatares, tooltips,
  menus de contexto, links e imagens. Os próprios elementos são templates JSON
  (`lib/*.json`), e seus componentes são escritos do mesmo jeito.
- **Layout no espírito do Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex,
  rolagem, listas virtualizadas para dezenas de milhares de linhas e `Flow` — um parágrafo
  de trechos de texto inline com quebra de linha entre eles.
- **Bindings e estado**: caminhos nos dados (`@user.name`), interpolação de texto, estado
  local de um componente (`@~open`), seletores por estado (hover, pressed, focus), `switch` e
  algumas funções (`eq`, `not`, `len`, `ratio`…). Structs viram dados e vice-versa:
  `#[derive(Data)]` em Rust, tabelas de campos em C, reflexão sem macros em C++20.
- **Temas**: claro, escuro e "seguir o sistema", construídos a partir de tokens que você
  pode sobrescrever.
- **Desenho próprio**: um nó `Canvas` chama sua função para escrever primitivas no retângulo
  dele, ao lado dos elementos comuns.
- **Threads**: uma thread de trabalho envia resultados de volta para a thread da UI (`post`),
  em todas as plataformas exceto o navegador.
- **Live reload** em builds de debug: com `UISON_RELOAD=folder`, os arquivos JSON da tela são
  relidos quando você os salva.
- **Testes sem janela**: um cenário (`UISON_SCRIPT=scenario.json`) toca, digita, rola e tira
  screenshots pela `key` dos elementos; os mesmos cenários rodam em todas as plataformas, no
  Chrome headless, no simulador de iOS e em um celular.
- **API de C e C++**: `uison.h` (C99), `uison_kit.h` (utilitários para C), `uison.hpp` (C++17,
  com facilidades em C++20) e `uison_widgets.hpp` (widgets C++20 gerados a partir do mesmo
  schema da DSL em Rust); bibliotecas estáticas e dinâmicas, um pacote CMake e `pkg-config`.
  C e C++ também compilam para o navegador com Emscripten: `em++ main.cpp libuison.a -o index.html`.

## Plataformas

| plataforma | backend | testado em |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO via runtime do Objective-C | Mac com Apple Silicon — a principal máquina de desenvolvimento |
| iOS | UIKit, CADisplayLink, o mesmo código CoreGraphics/CoreText do macOS | simulador e iPhone 12 mini |
| Android | JNI à mão, `android.graphics.Canvas`, uma camada Java de ~300 linhas; APK gerado sem Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite e WIC, COM à mão; DPI por monitor, barra de título escura | PC com Windows 11 em 100/125/150 % e dois monitores com escalas diferentes |
| Linux | código próprio do protocolo Wayland e X11 (xcb), Cairo + Pango, xkbcommon — tudo via `dlopen`, então o binário depende só da libc | Mutter, KWin, Sway, Weston e X11 headless (arm64 e x86_64); Ubuntu 24.04 com GNOME em uma máquina virtual |
| Navegador | `wasm32-unknown-unknown` sem wasm-bindgen, canvas 2D; C/C++ via Emscripten | Chrome (e Chrome headless) |

Ainda não testado: uma tela sensível ao toque de verdade no Windows, Windows on ARM (compila,
mas ainda não foi executado), Linux em hardware real e KDE, entrada de mouse e teclado no
GNOME, Safari e Firefox.

## Tamanho e velocidade

Builds de release (`opt-level = "s"`, LTO, `panic = "abort"`, sem símbolos). Os tamanhos vêm
das medições do próprio projeto ao longo do seu crescimento, então os builds mais recentes
podem diferir em alguns por cento.

| programa | tamanho |
|---|---|
| lição 1 (o contador acima), Rust, macOS | ~0,7 MB |
| o mesmo em C / C++ com `libuison.a`, macOS | 672 KB / 816 KB |
| Studio — a demo principal (e-mail com 2.000 mensagens, kanban, contatos, gráficos, formulários), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, sem precisar do Visual C++ Redistributable |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, navegador (`.wasm`, gzip) | ~220 KB |

| medição | resultado |
|---|---|
| evento → janela pintada, macOS | ~0,8 ms, dos quais ~0,2 ms são trabalho do próprio uison |
| toque → frame, iPhone 12 mini | 3,6 ms; rolagem de lista a 60 fps constantes |
| repintura completa do Studio, iPhone 12 mini / Huawei P40 lite | 0,7 ms / 0,4–0,7 ms |
| evento de mouse → frame, Windows (Iris Xe, 60 Hz) | p50 1,4–1,7 ms |
| evento → frame, Chrome | mediana 2,2 ms |
| inicialização → primeiro frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, navegador 24–42 ms |

O [Disk Usage](../../examples/disk-usage/README.md), um port de um programa feito em egui,
permite uma comparação direta: no macOS arm64 o binário tem 857 KB em vez de 5,1 MB, sem
dependências em vez de 362 crates no `Cargo.lock`, e com a mesma velocidade de varredura e
o mesmo uso de memória.

## Documentação

A documentação está em inglês, com uma [versão em russo](../ru/README.md) de cada página.

- [Escolha uma linguagem](../README.md) — por onde começar.
- Séries de lições, cada uma legível por si só, da preparação do sistema até threads e seu
  próprio frame: [Rust](../rust/README.md), [C](../c/README.md), [C++](../cpp/README.md).
  As lições principais são o mesmo programa nas três linguagens, e os screenshots delas
  batem pixel a pixel.
- [A linguagem de telas](../screens/README.md): [bindings e valores](../screens/bindings.md),
  [layout](../screens/layout.md), [elementos](../screens/elements.md),
  [temas](../screens/themes.md).
- [iOS e Android](../mobile.md), [o navegador (WebAssembly)](../wasm.md).

## Exemplos

| exemplo | o que mostra |
|---|---|
| `studio` | a demo principal: e-mail com 2.000 mensagens, kanban, contatos, gráficos animados, um formulário, configurações; layout de celular abaixo de 700 pt |
| `gallery` | todos os elementos da biblioteca |
| `disk-usage` | um port de um app em egui: gráfico sunburst desenhado em um nó `Canvas`, painéis e menus como tela do uison, um scanner paralelo sobre `std` |
| `browser` | um pequeno navegador web: parser próprio de HTML e CSS com cascata, HTTP e HTTPS via TLS do sistema; uma página vira uma tela do uison |
| `todo` | uma lista de tarefas na DSL em Rust |
| `list10k` | uma lista virtualizada de 10.000 linhas |
| `form2`, `settings` | telas de formulários e de configurações |
| `demo`, `scroll`, `uibench` | primitivas à mão, rolagem, um benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

Os exemplos em C e C++ ficam em [`examples-c`](../../examples-c) e [`examples-cpp`](../../examples-cpp),
cada um um projeto CMake independente.

## Build e testes

Você precisa do Rust stable; nada mais no macOS, Windows (com o toolchain MSVC) e Linux (sem
pacotes `-dev`: as bibliotecas do sistema são carregadas em tempo de execução). A preparação
de cada sistema é a lição 0 de cada série ([Rust](../rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

No Windows os scripts são `tools\*.ps1`: `tools\win.ps1 studio` compila, roda um cenário e
salva screenshots; `tools\c-dist.ps1 -Test` compila e testa o pacote C/C++.

Opções do backend para depuração, iguais em todo lugar: `UISON_STATS=1` imprime os tempos de
eventos, frames e animação, `UISON_FLASH=1` destaca as regiões repintadas, e
`UISON_SCRIPT` / `UISON_CAPTURE` rodam um cenário e salvam screenshots.

## Estrutura

```
crates/
  uison-json      parser JSON pull e árvore de valores, com comentários, vírgulas finais e chaves sem aspas
  uison-core      geometria, cores, fontes, eventos, os traits Canvas e Host, o trait App
  uison-frame     o frame de primitivas: parsing, diff → dano, hit testing, o executor de cenários
  uison-ui        o compilador de telas: templates, bindings, temas, layout, o runtime, a DSL em Rust, lib/*.json
  uison-apple     compartilhado por macOS e iOS: FFI de Objective-C, CoreGraphics, CoreText, ImageIO
  uison-macos     backend AppKit
  uison-ios       backend UIKit
  uison-android   backend JNI e sua camada Java (java/dev/uison)
  uison-windows   backend Win32 + Direct2D + DirectWrite
  uison-linux     backend Wayland e X11, Cairo + Pango, tudo via dlopen
  uison-web       backend WebAssembly (web/uison.js)
  uison-c         a API de C e os headers de C++, biblioteca estática e dinâmica, pacote CMake
  uison-gen       gera os widgets C++, a DSL em Rust e a referência de elementos a partir do schema de telas
  uison-derive    #[derive(Data)] sem syn
  uison           o crate guarda-chuva: tudo para a plataforma atual, uison::prelude; as lições de Rust
examples/         exemplos em Rust (veja acima)
examples-c/       exemplos e lições em C, um projeto CMake
examples-cpp/     exemplos e lições em C++, um projeto CMake
scenarios/        cenários de teste compartilhados por todas as plataformas
tools/            scripts de build, execução e testes para cada plataforma; drivers de entrada real de mouse e teclado
web/              a página e o lado JavaScript do backend WebAssembly
docs/             documentação em inglês; ru/, a mesma em russo; readme/, este README em outros idiomas
```

## Limitações

- Uma janela por programa; sem arrastar e soltar.
- A entrada de texto cobre o alfabeto latino e o cirílico e os teclados virtuais dos
  celulares; a entrada composta para chinês, japonês e coreano (IME) no desktop não está
  pronta, nem o texto da direita para a esquerda.
- Ainda sem suporte a acessibilidade (leitores de tela).
- Sem threads no navegador.
- O framework é jovem: versão 0.1, a API pode mudar.

## Traduções

A documentação está em inglês, e `docs/ru/` traz as mesmas páginas em russo, com um seletor
de idioma no topo de cada uma. Este README também está traduzido:
`docs/readme/README.<code>.md`, um por idioma, com a lista de idiomas no topo de cada um.
Uma alteração aqui também deve ir para as traduções.

## Licença

A licença ainda não foi escolhida.
