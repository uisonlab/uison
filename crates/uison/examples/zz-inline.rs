use uison::prelude::*;

fn main() {
    let screen = row().padding(20).gap(8).cross("center").children([
        button("−").on_click(|cx| cx.data.set("count", cx.data.num("count") - 1.0)),
        text("@count").size(18),
        button("+").on_click(|cx| cx.data.set("count", cx.data.num("count") + 1.0)),
    ]);
    let mut ui = Ui::from_node(screen).with_size(300.0, 100.0);
    ui.data.set("count", 0);
    run(ui, "Counter");
}
