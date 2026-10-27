// Dynamic library name for the loader: @rpath/libuison.dylib (Apple), libuison.so (Linux) —
// otherwise the program records a path inside target/ and won't start on another machine.
fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "macos" | "ios" => println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libuison.dylib"),
        "linux" | "android" => println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libuison.so"),
        _ => {}
    }
}
