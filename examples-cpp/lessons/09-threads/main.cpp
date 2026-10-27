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
