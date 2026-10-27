# Урок 6. Темы и токены

[English](../../../rust/lessons/06-themes.md) · **Русский**

**Что будет:** вкладки «Light / Dark / System» и пять кружков-цветов: выбор перекрашивает флажок, ползунок и кнопку.

![Урок 6](../../../rust/img/06-themes.png)

<!-- code: crates/uison/examples/06-themes.rs -->
```rust
//! Lesson 6: themes and tokens. Element colors are theme tokens (`$theme.primary`, `$theme.card`…);
//! `cx.set_theme` changes the theme, `cx.set_token` a single token.
use uison::prelude::*;

fn main() {
    let accents = row().gap(8).children([for_each(
        "accents",
        "c",
        container().key("accent-{@c}").w(32).h(32).radius(16).fill("@c").on_click(|cx| {
            // key "accent-#34C759" → the color after the dash.
            let color = cx.key.rsplit('-').next().unwrap_or("").to_string();
            cx.set_token("primary", color.as_str());
            cx.data.set("accent", color);
        }),
    )]);
    let screen = column().padding(20).gap(14).cross("stretch").children([
        tabs()
            .key("theme")
            .bind("theme")
            .on_change(|cx| {
                let t = cx.data.str("theme").to_string();
                cx.set_theme(&t);
            })
            .tabs(&[("light", "Light"), ("dark", "Dark"), ("system", "System")]),
        accents,
        card([
            checkbox("Checkbox").bind("agree"),
            slider().bind("volume").min(0).max(100),
            row().gap(8).children([button("Primary"), button("Secondary").variant("secondary")]),
        ])
        .title("Example"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(440.0, 380.0);
    ui.set_theme("light");
    ui.data.set("theme", "light");
    ui.data.set("accent", "#007AFF");
    ui.data.set("agree", true);
    ui.data.set("volume", 60);
    ui.data.set("accents", vec!["#007AFF", "#34C759", "#FF9500", "#AF52DE", "#FF2D55"]);
    run(ui, "Lesson 6 — themes");
}
```

## Как это работает

1. **Токены.** Элементы библиотеки берут цвета не напрямую, а из токенов темы: `$theme.primary`, `$theme.card`, `$theme.text`… Сменили тему — перекрасилось всё. Список токенов — [themes.md](../../screens/themes.md).
2. **Тема целиком.** `cx.set_theme("dark")` в обработчике, `ui.set_theme("light")` до запуска. `"system"` — как в системе, и экран сам переключится, когда пользователь сменит тему ОС.
3. **Один токен.** `cx.set_token("primary", "#AF52DE")` меняет главный цвет поверх темы — флажок, ползунок, кнопка «Primary» перекрашиваются сразу.
4. **Цвет из данных.** `.fill("@c")` у кружка — привязка: цвет берётся из элемента массива `accents`.
5. **Вкладки.** `tabs().bind("theme").tabs(&[…])` — выбор пишет в `theme`, `on_change` вызывается после.

## Попробуйте

- Запоминать выбор между запусками: записать тему в файл в `on_change` и прочитать при старте.
- Свой токен: `ui.data` не нужен — `cx.set_token("radius", 2)` делает все элементы почти квадратными.

Дальше — [урок 7: картинки, подсказки, меню](07-rich.md).
