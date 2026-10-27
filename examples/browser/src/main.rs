//! uison browser: a window on macOS, Windows, Linux; phones and the web go through tools/*.sh.

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if native::debug_modes() {
        return;
    }
    let start = std::env::args().nth(1).filter(|a| !a.starts_with('-')).unwrap_or_else(|| browser::HOME.to_string());
    uison::run(browser::app_at(&start), "uison browser");
}

/// Debug modes: `--json address` — the page's JSON; `--png dir [address…]` — screenshots (macOS, Windows, Linux).
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--json" || a == "--png") {
            browser::SYNC.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(i) = args.iter().position(|a| a == "--json") {
            let url = args.get(i + 1).map_or(browser::HOME, String::as_str);
            let src = browser::fetch(url);
            let env = browser::style::Env { width: 1100.0, height: 800.0, dark: false };
            let r = browser::render(&src, env, "p");
            println!("{}", browser::pretty::pretty(&r.page.root));
            eprintln!("{} nodes; parse {} µs, CSS {} µs, cascade {} µs, convert {} µs", r.page.nodes, r.us[0], r.us[1], r.us[2], r.us[3]);
            return true;
        }
        #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
        if args.iter().any(|a| a == "--bench") {
            bench(&args);
            return true;
        }
        #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
        if let Some(i) = args.iter().position(|a| a == "--png") {
            let dir = &args[i + 1];
            std::fs::create_dir_all(dir).unwrap();
            let urls: Vec<&str> = args[i + 2..].iter().map(String::as_str).collect();
            let urls = if urls.is_empty() { vec!["about:home", "about:css", "about:article", "about:retro"] } else { urls };
            for (n, url) in urls.iter().enumerate() {
                // Window height comes from UISON_PNG_H (2400 by default) so the whole page is visible.
                let height = std::env::var("UISON_PNG_H").ok().and_then(|h| h.parse().ok()).unwrap_or(2400.0);
                let h = uison::Headless::new(browser::app_at(url).with_size(1100.0, height), 2.0).unwrap();
                let name = url.rsplit([':', '/']).find(|s| !s.is_empty()).unwrap_or("page");
                h.save_png(&format!("{dir}/{}_{name}.png", n + 1), false).unwrap();
            }
            return true;
        }
        false
    }

    /// `--bench [words] [address…]`: page conversion and frame build (median of 30).
    #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
    fn bench(args: &[String]) {
        use std::time::Duration;
        if args.iter().any(|a| a == "words") {
            browser::convert::WORDS.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let urls: Vec<&str> = args.iter().skip(1).map(String::as_str).filter(|a| !a.starts_with("--") && *a != "words").collect();
        let urls = if urls.is_empty() { vec!["about:home", "about:css", "about:article", "article×10", "about:retro"] } else { urls };
        let text = uison::Text::new();
        let env = browser::style::Env { width: 1100.0, height: 800.0, dark: false };
        println!("{:<14} {:>6} {:>6} | {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} | {:>7} {:>7} {:>7} {:>7}", "page", "HTML", "nodes", "parse", "style", "conv", "nodes", "prims", "bytes", "expand", "layout", "emit", "frame");
        let med = |mut v: Vec<Duration>| {
            v.sort();
            v[v.len() / 2].as_micros()
        };
        for url in urls {
            let src = if url == "article×10" {
                // The same article with its body ×10 (~120 KB of HTML).
                let a = browser::builtin("about:article").unwrap();
                let (start, end) = (a.find("<main>").unwrap() + 6, a.find("</main>").unwrap());
                let html = format!("{}{}{}", &a[..start], a[start..end].repeat(10), &a[end..]);
                browser::Source { url: url.into(), html, css: Vec::new() }
            } else {
                browser::fetch(url)
            };
            let rs: Vec<browser::Rendered> = (0..10).map(|_| browser::render(&src, env, "b")).collect();
            let us = |k: usize| {
                let mut v: Vec<u32> = rs.iter().map(|r| r.us[k]).collect();
                v.sort();
                v[5]
            };
            let page = &rs[0].page;
            let mut ui = browser::app_at("about:blank");
            if let Some(mut p) = ui.tree().find("page") {
                *p.children() = vec![page.root.clone()];
            }
            ui.render(&text);
            let mut ex = Vec::new();
            let mut lay = Vec::new();
            let mut em = Vec::new();
            let mut fr = Vec::new();
            for _ in 0..30 {
                let t = std::time::Instant::now();
                ui.render(&text);
                fr.push(t.elapsed());
                ex.push(ui.stats.expand);
                lay.push(ui.stats.layout);
                em.push(ui.stats.emit);
            }
            let st = ui.stats;
            println!(
                "{:<14} {:>5}K {:>6} | {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} | {:>7} {:>7} {:>7} {:>7}",
                url.trim_start_matches("about:").trim_start_matches("http://"),
                src.html.len() / 1024,
                page.nodes,
                us(0),
                us(1) + us(2),
                us(3),
                st.nodes,
                st.prims,
                st.bytes,
                med(ex),
                med(lay),
                med(em),
                med(fr)
            );
        }
    }
}
