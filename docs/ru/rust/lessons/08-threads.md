# Урок 8. Потоки

[English](../../../rust/lessons/08-threads.md) · **Русский**

**Что будет:** «скачивание» в рабочем потоке: полоса прогресса, проценты, кнопка «Cancel». Интерфейс не замирает.

![Урок 8](../../../rust/img/08-threads.png)

<!-- code: crates/uison/examples/08-threads.rs -->
```rust
//! Lesson 8: work in another thread. The UI is touched only through `Handle::post`: the function
//! runs on the UI thread (like a handler), and the screen updates itself. Cancellation is an atomic flag.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use uison::prelude::*;

fn main() {
    let screen = column().padding(24).gap(14).cross("stretch").children([
        text("Downloading").size(20).bold(),
        progress("@progress"),
        text("@status").color("$theme.muted"),
        row().gap(8).children([
            // Actions by name: the handlers need a handle, which only a constructed Ui has.
            button("Download").key("start").icon("arrowDown").disabled("@busy").on("click", "start"),
            button("Cancel").key("cancel").variant("secondary").visible("@busy").on("click", "cancel"),
        ]),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 240.0);
    ui.data.set("progress", 0);
    ui.data.set("busy", false);
    ui.data.set("status", "Press “Download”");
    let handle = ui.handle(); // cloned into threads
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    ui.on("start", move |cx| {
        cx.data.set("busy", true);
        stop.store(false, Ordering::Relaxed);
        let (h, stop) = (handle.clone(), stop.clone());
        std::thread::spawn(move || {
            for i in 1..=40 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50)); // "download" a chunk
                h.post(move |cx| {
                    cx.data.set("progress", i as f64 / 40.0);
                    cx.data.set("status", format!("Downloaded {}%", i * 25 / 10));
                });
            }
            let stopped = stop.load(Ordering::Relaxed);
            h.post(move |cx| {
                cx.data.set("busy", false);
                cx.data.set("status", if stopped { "Cancelled" } else { "Done" });
            });
        });
    });
    ui.on("cancel", move |_| cancel.store(true, Ordering::Relaxed));
    run(ui, "Lesson 8 — threads");
}
```

## Как это работает

1. **Интерфейс — в одном потоке.** Данные, дерево, обработчики живут в потоке окна. Долгая работа (сеть, файлы, вычисления) — в другом потоке, иначе окно не отвечает.
2. **`Handle::post`.** `ui.handle()` — ссылка на интерфейс, которую можно передавать в потоки (`Send + Sync`, клонируется). `h.post(|cx| …)` ставит замыкание в очередь: оно выполнится в потоке окна как обычный обработчик, с `cx.data`, и экран обновится.
3. **Отмена.** Флаг `AtomicBool` общий у кнопки «Cancel» и потока; поток проверяет его на каждом шаге.
4. **Действия по имени.** Замыканию нужен `handle`, а он есть только у созданного `Ui`, поэтому кнопки здесь называют действия (`.on("click", "start")`), и обработчики регистрируются после `Ui::from_node`.
5. **Браузер.** В WebAssembly потоков нет, `post` работает из того же потока (из таймеров, ответов сети).

## Попробуйте

- Настоящая загрузка: `std::net::TcpStream` или HTTP-клиент в потоке, прогресс — по прочитанным байтам.
- Несколько загрузок сразу: список в данных, у каждой — свой прогресс (`progress("@d.progress")` в `for_each`).

Дальше — [урок 9: тесты экрана](09-tests.md).
