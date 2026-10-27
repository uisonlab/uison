// Handlers referencing main's local variables ([&]) live as long as the program runs.
// In the browser uison_run doesn't return from main and doesn't unwind the stack, with any exception mode
// (-fwasm-exceptions, -fexceptions, none). Scenario: locals.json; the text reads "start … click 3, n = 3".
#include <string>
#include <vector>

#include <uison.hpp>

int main() {
    int n = 0;
    std::vector<std::string> log{"start"};
    uison::Ui ui(R"json({ size: [420, 160], data: { text: "" }, root: { Column: { padding: 24, gap: 12, children: [
      { Text: { text: "@text", size: 20 } },
      { Button: { key: "inc", text: "+1", on: { click: "inc" } } } ] } } })json");
    ui.on("inc", [&](uison::Cx& cx) {
        n++;
        log.push_back("click " + std::to_string(n));
        cx.data().set("text", log.front() + " … " + log.back() + ", n = " + std::to_string(n));
    });
    return ui.run("locals");
}
