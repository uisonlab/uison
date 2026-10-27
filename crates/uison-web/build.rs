// Emscripten: all of web/uison.js sits in libuison.a as an EM_JS string (body of uison_js_boot) and stays
// in program memory — store a copy without comments and indentation (emcc minifies its own JS later).
fn main() {
    let src = "../../web/uison.js";
    println!("cargo:rerun-if-changed={src}");
    let js = std::fs::read_to_string(src).expect("web/uison.js");
    let mut out = String::with_capacity(js.len());
    for line in js.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        out.push_str(t);
        out.push('\n');
    }
    let dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{dir}/uison.min.js"), out).unwrap();
}
