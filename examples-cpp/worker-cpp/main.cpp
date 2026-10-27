// Work in another thread: a "download" with progress. The UI is touched only through handle.post:
// the function runs on the UI thread, and the frame updates itself.
#include <chrono>
#include <thread>

#include <uison.hpp>

static const char* SCREEN = R"json({
  size: [420, 220],
  data: { progress: 0, busy: false, status: "Press “Load”" },
  root: { Center: { child: { Column: { w: 340, gap: 14, cross: "stretch", children: [
    { Text: { text: "Loading", size: 20, bold: true } },
    { ProgressBar: { value: "@progress" } },
    { Text: { text: "@status", color: "$theme.muted" } },
    { Button: { key: "start", text: "Load", icon: "arrowDown", disabled: "@busy", on: { click: "start" } } },
  ] } } } }
})json";

int main() {
    uison::Ui ui(SCREEN);
    uison::Handle handle = ui.handle();
    ui.on("start", [handle](uison::Cx& cx) {
        cx.data().set("busy", true);
        // The worker thread "downloads" 20 chunks and reports progress.
        std::thread([handle] {
            for (int i = 1; i <= 20; i++) {
                std::this_thread::sleep_for(std::chrono::milliseconds(60));
                handle.post([i](uison::Cx& cx) {
                    cx.data().set("progress", i / 20.0);
                    cx.data().set("status", "Loaded " + std::to_string(i * 5) + "%");
                });
            }
            handle.post([](uison::Cx& cx) {
                cx.data().set("busy", false);
                cx.data().set("status", "Done");
            });
        }).detach();
    });
    return ui.run("Loading (C++)");
}
