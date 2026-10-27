# Lesson 9. Threads

**English** · [Русский](../../ru/cpp/lessons/09-threads.md)

**What you'll get:** a "download" in a worker thread: a progress bar, a percentage and a "Cancel" button. The interface doesn't freeze.

![Lesson 9](../img/09-threads.png)

<!-- code: examples-cpp/lessons/09-threads/main.cpp -->
```cpp
// Lesson 9: work in another thread. The UI is touched only through Handle::post: the function runs
// on the UI thread (like a handler), and the screen updates itself. Cancellation is std::stop_token (C++20).
#include <chrono>
#include <string>
#include <thread>

#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    auto screen = Column({.padding = 24, .gap = 14, .cross = "stretch", .children = {
        Text("Downloading", {.size = 20, .bold = true}),
        ProgressBar({.value = "@progress"}),
        Text("@status", {.color = "$theme.muted"}),
        Row({.gap = 8, .children = {
            // Actions by name: the handlers need a handle, which only a constructed Ui has.
            Button("Download", {.icon = "arrowDown", .key = "start", .disabled = "@busy", .on_click = "start"}),
            Button("Cancel", {.variant = "secondary", .key = "cancel", .visible = "@busy", .on_click = "cancel"}),
        }}),
    }});
    uison::Ui ui(screen, {.size = {420, 240}, .data = uison::obj{{"progress", 0}, {"busy", false}, {"status", "Press “Download”"}}});
    uison::Handle handle = ui.handle(); // copied into threads
    std::jthread worker;                // lives in main: when the window closes, the thread is stopped and joined
    ui.on("start", [&](uison::Cx& cx) {
        cx.data().set("busy", true);
        worker = std::jthread([handle](std::stop_token stop) {
            for (int i = 1; i <= 40 && !stop.stop_requested(); i++) {
                std::this_thread::sleep_for(std::chrono::milliseconds(50)); // "download" a chunk
                handle.post([i](uison::Cx& cx) {
                    cx.data().set("progress", i / 40.0);
                    cx.data().set("status", "Downloaded " + std::to_string(i * 25 / 10) + "%");
                });
            }
            bool stopped = stop.stop_requested();
            handle.post([stopped](uison::Cx& cx) {
                cx.data().set("busy", false);
                cx.data().set("status", stopped ? "Cancelled" : "Done");
            });
        });
    });
    ui.on("cancel", [&] { worker.request_stop(); });
    return ui.run("Lesson 9 — threads");
}
```

## How it works

1. **The interface lives in one thread.** Data, the tree and handlers live in the window's thread. Long-running work goes to another thread, otherwise the window stops responding.
2. **`Handle::post`.** `ui.handle()` is a reference to the interface for other threads (copyable; take it before `run`). `handle.post([](uison::Cx& cx) { … })` queues a lambda: it runs in the window's thread like a regular handler, and the screen updates.
3. **`std::jthread` and cancellation.** The thread receives a `std::stop_token`; the "Cancel" button calls `worker.request_stop()`. The `jthread` lives in `main`: when the window is closed, its destructor asks the thread to stop and waits for it. Before LLVM 20 (Xcode 16 and older) libc++ has `jthread` only with `-fexperimental-library`; `examples-cpp/CMakeLists.txt` checks and adds the flag itself.
4. **Lambdas with references into `main`.** `[&]` captures `worker` and `handle` from `main` — they live as long as the window runs (in the browser too: `main` isn't unwound there).
5. **Actions by name.** Handlers need `handle`, which only a constructed `Ui` has, so the buttons name their actions (`.on_click = "start"`).
6. **Browser.** WebAssembly has no threads: this lesson isn't built there, and `post` works from the same thread.

## Try it

- A real download: an HTTP client in the thread, progress by bytes.
- Several downloads at once: a vector of structs in the data and `For` with a `ProgressBar` in the template.

Next: [lesson 10: your own frame](10-frame.md).
