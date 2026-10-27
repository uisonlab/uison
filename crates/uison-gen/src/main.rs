//! `uison-gen` — wrappers from the screen schema (see the `uison_gen` library).

fn main() {
    let mut args = std::env::args().skip(1);
    let mut ns = "uison::widgets".to_string();
    let mut rust = false;
    let mut md = false;
    let mut out_file: Option<String> = None;
    let mut files = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--namespace" => ns = args.next().unwrap_or_else(|| usage()),
            "--rust" => rust = true,
            "--md" => md = true,
            "-o" => out_file = Some(args.next().unwrap_or_else(|| usage())),
            "-h" | "--help" => usage(),
            _ => files.push(a),
        }
    }
    // Output goes to stdout or a file (-o: CMake, build.rs); the file isn't rewritten if unchanged.
    let emit = |s: String| match &out_file {
        Some(f) => {
            if std::fs::read_to_string(f).ok().as_deref() != Some(s.as_str()) {
                std::fs::write(f, s).unwrap_or_else(|e| fail(&format!("{f}: {e}")));
            }
        }
        None => print!("{s}"),
    };
    if md {
        emit(uison_gen::md::elements());
        return;
    }
    if files.is_empty() {
        emit(if rust { uison_gen::rust::standard() } else { uison_gen::standard_header() });
        return;
    }
    let srcs: Vec<String> =
        files.iter().map(|f| std::fs::read_to_string(f).unwrap_or_else(|e| fail(&format!("{f}: {e}")))).collect();
    let refs: Vec<&str> = srcs.iter().map(String::as_str).collect();
    let what = files.join(", ");
    let out = if rust {
        let name = if ns == "uison::widgets" { "Widgets".to_string() } else { ns.clone() };
        uison_gen::rust::components(&refs, &name, &what)
    } else {
        uison_gen::components_header(&refs, &ns, &what)
    };
    emit(out.unwrap_or_else(|e| fail(&format!("{what}:{e}"))));
}

fn usage() -> ! {
    eprintln!("usage: uison-gen                                   > uison_widgets.hpp  (standard C++ widgets)");
    eprintln!("       uison-gen [--namespace app] components.json… > app_widgets.hpp  (your components, C++)");
    eprintln!("       uison-gen --rust                            > widgets.rs         (standard Rust DSL)");
    eprintln!("       uison-gen --rust [--namespace AppProps] components.json… > app.rs (your components, Rust)");
    eprintln!("       uison-gen --md                              > elements.md        (reference)");
    eprintln!("       -o file — write to a file (unchanged output keeps the file's time)");
    std::process::exit(2)
}

fn fail(msg: &str) -> ! {
    eprintln!("uison-gen: {msg}");
    std::process::exit(1)
}
