# Урок 7. Картинки, подсказки, меню

[English](../../../rust/lessons/07-rich.md) · **Русский**

**Что будет:** карточка с фото, ссылкой и кнопкой с подсказкой; правой кнопкой (долгим нажатием на телефоне) — меню; ниже — картинка.

![Урок 7](../../../rust/img/07-rich.png)

![Меню](../../../rust/img/07-rich-menu.png)

<!-- code: crates/uison/examples/07-rich.rs -->
```rust
//! Lesson 7: images, tooltips, a context menu, links, the clipboard. Everything except images
//! is just properties; an image is given as bytes (PNG or JPEG): `include_bytes!` embeds the file into the program.
use uison::prelude::*;

const SITE: &str = "https://github.com/MarketKernel/uison";

fn main() {
    let card = row()
        .key("card")
        .gap(14)
        .cross("center")
        .padding(12)
        // Context menu: label, action; "-" is a separator. Actions are called by name (ui.on below).
        .context_menu(json(r#"[{ label: "Copy name", action: "copy" }, "-", { label: "Open website", action: "site" }]"#))
        .children([
            avatar("Alice Parker").src("@photo").size(56),
            column().gap(4).flex(1).children([
                text("Alice Parker").bold().size(16),
                link("github.com/MarketKernel/uison").href(SITE),
            ]),
            // Tooltip: on any node (appears after half a second of hovering).
            icon_button("edit").key("copy").tooltip("Copy name").on("click", "copy"),
        ]);
    let screen = column().padding(20).gap(14).cross("stretch").children([
        card,
        // Image: the size is given or natural (one side given, the other keeps the aspect ratio);
        // fit: cover (crop), contain (fit inside), fill (stretch).
        image("@landscape").h(90).fit("cover").radius(12),
        text("@status").color("$theme.muted"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(440.0, 300.0);
    // Images from bytes → source "mem:photo"; the file path is relative to this lesson file.
    let photo = ui.image("photo", include_bytes!("../../../examples/gallery/images/anna.jpg").as_slice());
    let landscape = ui.image("landscape", include_bytes!("../../../examples/gallery/images/landscape.png").as_slice());
    ui.data.set("photo", photo);
    ui.data.set("landscape", landscape);
    ui.data.set("status", "Right-click (or long-press) the card for a menu");
    ui.on("copy", |cx| {
        cx.copy("Alice Parker");
        cx.data.set("status", "Name copied to clipboard");
    });
    ui.on("site", |cx| cx.open_url(SITE));
    run(ui, "Lesson 7 — images and menus");
}
```

## Как это работает

1. **Картинки из байтов.** `ui.image("photo", bytes)` регистрирует PNG или JPEG и возвращает источник `"mem:photo"` — его кладут в данные и привязывают: `avatar(…).src("@photo")`, `image("@landscape")`. `include_bytes!` встраивает файл в программу. Файл рядом с программой — путём: `image("photo.jpg")`; в браузере — и адресом `https://…`.
2. **Размер картинки.** Задан один размер (`h(90)`) — второй по пропорции; `fit("cover")` обрезает, `"contain"` вписывает, `"fill"` растягивает. `radius` скругляет углы.
3. **Подсказка.** `.tooltip("…")` у любого узла: появляется через полсекунды наведения, на телефоне — по долгому нажатию.
4. **Контекстное меню.** `.context_menu(json(…))` — список `{ label, action }`, `"-"` — разделитель. Пункт вызывает действие по имени (`ui.on("copy", …)`), `cx.key` — узел, на котором открыли меню.
5. **Ссылка.** `link("text").href("https://…")` открывает адрес в браузере; из кода — `cx.open_url(…)`.
6. **Буфер обмена.** `cx.copy("…")` — в системный буфер; вставка в поля ввода работает сама (⌘V / Ctrl+V, меню поля).

## Попробуйте

- Пункт меню «Delete» с подтверждением: второе меню или `visible` у панели.
- Аватар без фото: уберите `.src(…)` — останутся инициалы на цветном кружке (цвет — по имени).

Дальше — [урок 8: потоки](08-threads.md).
