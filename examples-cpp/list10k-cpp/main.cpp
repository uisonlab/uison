// A virtual list of 10,000 rows (like examples/list10k). The data is a vector of structs.
// `list10k-cpp --bench`: three ways to pass 10,000 rows across the boundary, headless.
#include <chrono>
#include <cstdio>
#include <cstring>
#include <optional>
#include <uison.hpp>

struct Message {
    std::string name;
    std::optional<std::string> photo;
    std::string text;
    std::string time;
};

static const char* NAMES[] = {"Anna Peterson", "Ben Irving", "Clara Smith", "David Owens", "Emma Cole", "Frank White", "Grace Fox", "Henry Kim"};
static const char* TEXTS[] = {
    "Hi! How is the project going?",
    "Sent over the mockups, take a look when you have time",
    "Is the 15:00 call still on?",
    "Great, thanks 👍",
    "Found a rendering bug, fixing it now",
};

static std::vector<Message> messages(const std::vector<std::string>& photos) {
    std::vector<Message> v;
    v.reserve(10000);
    char t[8];
    for (int i = 0; i < 10000; i++) {
        std::snprintf(t, sizeof t, "%02d:%02d", 9 + i % 12, (i * 7) % 60);
        std::optional<std::string> photo;
        if (std::size_t(i % 8) < photos.size()) photo = photos[i % 8];
        v.push_back({NAMES[i % 8], photo, "#" + std::to_string(i) + ": " + TEXTS[i % 5], t});
    }
    return v;
}

static uison::Node screen() {
    auto tile = uison::list_tile("@m.name").subtitle("@m.text")
        .leading(uison::avatar("@m.name").src("@m.photo"))
        .trailing(uison::text("@m.time").size(12).color("$theme.muted"))
        .on_click("open");
    return uison::column().fill_w().fill_h().cross("stretch").children({
        uison::container().padding(uison::arr{14, 20}).child(uison::row().cross("center").gap(8).children({
            uison::text("Messages").size(20).bold(),
            uison::badge("{@count}").color("$theme.primary"),
            uison::spacer(),
            uison::text("@opened").size(12).color("$theme.muted"),
        })),
        uison::divider(),
        uison::list("messages", "m", 56, uison::container().padding(uison::arr{0, 8}).child(tile)).key("list").fill_w().flex(1),
    });
}

static double ms_since(std::chrono::steady_clock::time_point t0) {
    return std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t0).count();
}

// Three ways: a vector of structs (reflection → one JSON string), a ready JSON string, element by element.
static int bench() {
    uison::Ui ui(screen());
    auto d = ui.data();
    auto v = messages({});
    auto t0 = std::chrono::steady_clock::now();
    d.set("messages", v);
    double reflect = ms_since(t0);

    // The string is ready: only parsing on the uison side.
    std::string json = uison::Json(v).str();
    t0 = std::chrono::steady_clock::now();
    uison_data_set_json(uison_ui_data(ui.c()), "messages", json.c_str());
    double manual = ms_since(t0);

    t0 = std::chrono::steady_clock::now();
    d.remove("messages");
    for (auto& m : v) d.push("messages", m);
    double each = ms_since(t0);

    t0 = std::chrono::steady_clock::now();
    auto back = d.get<std::vector<Message>>("messages");
    double read = ms_since(t0);
    std::printf("10000 rows: set(vector) %.1f ms, set_json (ready string, %zu KB) %.1f ms, push x10000 %.1f ms; get<vector> %.1f ms (%zu)\n",
                reflect, json.size() / 1024, manual, each, read, back.size());
    return back.size() == 10000 ? 0 : 1;
}

int main(int argc, char** argv) {
    if (argc > 1 && std::strcmp(argv[1], "--bench") == 0) return bench();
    uison::Ui ui(screen(), {{"size", uison::arr{420, 720}}});
    // Photos for the first four names, initials for the rest (the files are next to the program).
    std::vector<std::string> photos;
    for (const char* n : {"anna", "boris", "vera", "gleb"}) photos.push_back(ui.image_file(n, std::string(n) + ".jpg"));
    auto v = messages(photos);
    auto d = ui.data();
    d.set("count", v.size());
    d.set("messages", v);
    d.set("opened", "");
    ui.on("open", [](uison::Cx& cx) {
        // The element key is "list/<index>/…": the row index is the second-to-last part.
        std::string k(cx.key());
        auto b = k.rfind('/');
        auto a = b == std::string::npos ? b : k.rfind('/', b - 1);
        cx.data().set("opened", "opened message " + (a == std::string::npos ? "?" : k.substr(a + 1, b - a - 1)));
    });
    return ui.run("10,000 rows (C++)");
}
