//! Lesson 1: a window with a counter. The screen is the Rust DSL, the handler is a closure right on the button.
use uison::prelude::*;

fn main() {
    // center → column → text + button: the same tree as { Center: { child: { Column: … } } } in JSON.
    let screen = center(column().gap(12).cross("center").children([
        // {@n} substitutes the value of n from data; the screen updates itself when n changes.
        text("Pressed {@n} times").size(20),
        button("+1").key("inc").on_click(|cx| cx.data.edit("n", |n: &mut i32| *n += 1)),
    ]));
    let mut ui = Ui::from_node(screen).with_size(320.0, 200.0);
    ui.data.set("n", 0);
    run(ui, "Lesson 1");
}
