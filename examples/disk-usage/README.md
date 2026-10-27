# Disk Usage on uison

A port of [Disk Usage](https://github.com/MarketKernel/disk-usage) (egui + rayon + rfd + trash):
where the disk space goes, as a sunburst chart. The scanner, the tree, the ring layout and the zoom
animation are the same; panels, list, breadcrumbs, menus, tooltips and About are a uison screen
(`app.json`, `components.json`); the chart is drawn by code in a `Canvas` node (`Ui::draw`).

```sh
cargo run --release -p disk-usage -- ~/Downloads         # window; without a path, the start screen
cargo run --release -p disk-usage -- --lang ru ~/Downloads   # start in Russian
target/release/disk-usage --png out/du ~/Documents        # headless screenshots: view, tooltip, selection, zoom, menu
target/release/disk-usage --bench ~/Documents             # frame build time
SCAN_BENCH=~/Documents cargo test --release -p disk-usage bench_scan -- --ignored --nocapture
```

## Icon

The picture is the original's (`assets/`: `icon.png` 1024 px, `icon-256.png`, `icon.ico` with 7 sizes), drawn by its
`src/icon.rs`. The running window gets `icon-256.png` through `uison::set_icon` (the Dock, the Windows title bar and
taskbar, `_NET_WM_ICON` on X11); the program file gets its icon from packaging:

```sh
examples/disk-usage/bundle.sh macos          # out/disk-usage/Disk Usage.app: .icns from icon.png, Info.plist
examples/disk-usage/bundle.sh linux          # ~/.local: the program, dev.uison.disk-usage.desktop, the hicolor icon
```

On Windows `build.rs` writes a `.res` from `icon.ico` (by hand, without a resource compiler) and gives it to the linker:
the icon is inside `disk-usage.exe`, and the backend uses it for the window. On Wayland the window's `app_id` is
`dev.uison.disk-usage` — the name of the `.desktop` file, which is how GNOME finds the icon.

## Languages

English (the default) and Russian; the EN / RU switch in the toolbar changes the language on the fly, `--lang ru`
starts in Russian. Fixed texts of the screen are bound to the data (`"@t.open"`), and the code puts the
dictionary there; phrases with names and numbers are built in code with Russian plural forms (1 файл, 2 файла,
5 файлов), and numbers follow the language: `8.2 GB` / `8,2 ГБ`. Everything is in `src/i18n.rs`, both languages
side by side.

## What uison needed

- **The `poly` primitive**: a closed polygon in every backend (`Canvas::poly`; `triangle` is a special
  case of it). A ring sector is a single polygon: triangles would leave antialiasing seams.
- **The `Canvas { draw: "name" }` node**: custom drawing. A program function writes primitives into the frame inside
  the node's rectangle (clipped). Everything else (layout, events, menus, tooltips) works as for any node.
- **`cx.pos`, `cx.clicks`, the `hover` event**: the cursor point in a handler (relative to the node), the click number
  in a series (double click), cursor movement over the node and leaving it (`pos = None`).
- **Re-entrancy protection** (macOS, Windows): a modal dialog inside a handler (folder picker) spins its own
  event loop; window events now wait for the handler to finish instead of entering the app a second time.

## Measurements (Mac, Apple Silicon, folder `~/Documents/GitHub`: 239 thousand files, 33 GB)

| | egui (original) | uison |
|---|---|---|
| macOS arm64 binary | 5,126,464 B | 856,864 B (**−83%**; 839,808 B before the Russian translation) |
| Linux aarch64 binary | — | 1,054,208 B |
| Windows x64 binary | — | 920,576 B (with the icon resource) |
| dependencies in Cargo.lock | 362 | 0 (std only) |
| scan (warm cache, 3 runs) | 1.65–2.02 s (rayon) | 1.69–2.32 s (own pool on std) |
| scanner peak memory | 50 MB | 56 MB |
| window memory after the scan | 119 MB | 115 MB |
| lines of Rust | 3,198 | 2,951 + 359 lines of screen JSON |

A frame with the chart (~3,000 sectors) is 181 KB of text. Hovering a sector takes ~2.5 ms from event to paint
(frame build 1.3 ms, parse 0.7 ms, redraw of 1–9% of the window); a zoom animation frame is a 1.8 ms build + 1.1 ms parse,
redrawing half the window; a full window redraw is 1.7 ms. 820 KB is almost entirely uison itself (Studio is the same
~820 KB): the example adds little with its own modules.

## Where it was tested

- **macOS**: window, real mouse and keyboard (`tools/drive.swift`): hover and tooltip, selection, double
  click on a sector and on a row (animated zoom), list scrolling, right click → "Copy path" (clipboard),
  ⌘⌫ → Trash queue and "Clear all", About and Escape, "Open…" (NSOpenPanel) with cancel.
  "Move to Trash" was not pressed: those are real folders (the code is `NSFileManager trashItemAtURL`).
- **Linux** (OrbStack, headless Sway): window with a scenario (quick folder, scan, About), `--png`
  screenshots (Cairo + Pango).
- **Windows** (Task 11, real mouse and keyboard via `windrive`): scan of the profile (722 thousand files: 8.1 s warm,
  17.6 s cold) and `C:\Windows\System32`, chart, tooltip, zoom, breadcrumbs, menu ("Show in Explorer", "Open", "Copy path"),
  folder dialog (`IFileOpenDialog`, modal to the window) with a typed path and with cancel, Trash queue on a temporary
  folder (`SHFileOperationW`, a file locked by another program stays in the queue with the system's message), EN/RU at
  100 % and 150 %, `--png`, the icon in Explorer, the title bar and the taskbar.

## Missing compared to the original

- Dragging a folder onto the window (uison has no drag-and-drop) and resizing panels with the mouse.
- The path in the window title (`Host` has no `set_title`), a minimum window size, the F5 key (`KeyCode` has no F1–F9).
- Scrolling the breadcrumbs: a long path is truncated.
- The `Canvas` node in the C/C++ API (no drawing function from C).
- Linux: folder picker via `zenity`/`kdialog`, Trash via `gio trash` (not tested on a real desktop).
