//! The simplest custom control: a function that returns a node built from ready elements.
use uison::prelude::*;

/// Stepper "− 3 +" bound to a number in the data.
fn stepper(path: &str) -> Node {
    let (dec, inc) = (path.to_string(), path.to_string());
    row().gap(8).cross("center").children([
        button("−").on_click(move |cx| cx.data.set(&dec, cx.data.num(&dec) - 1.0)),
        text(format!("@{path}")).size(18),
        button("+").on_click(move |cx| cx.data.set(&inc, cx.data.num(&inc) + 1.0)),
    ])
}

fn main() {
    let screen = column().padding(20).gap(12).children([stepper("apples"), stepper("pears")]);
    let mut ui = Ui::from_node(screen).with_size(300.0, 160.0);
    ui.data.set("apples", 3);
    ui.data.set("pears", 0);
    run(ui, "Stepper");
}
