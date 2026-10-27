# Lesson 0. Setting up your system

**English** · [Русский](../../ru/rust/lessons/00-setup.md)

**What you'll get:** Rust and everything it needs to build a window; at the end, lesson 1 running on your machine.

You need three things: **Rust** (installed with `rustup`), the **system linker** (Rust compiles the program, but the OS linker puts it together) and **git**. You don't need to install libraries for windows, graphics or fonts: uison takes them from the system — macOS and Windows always have them, and on Linux any desktop has them.

## macOS

1. Apple command line tools (linker, git):

   ```sh
   xcode-select --install
   ```

   If you have Xcode from the App Store, you already have them. You need the full Xcode only for the iPhone ([lesson 11](11-everywhere.md)).
2. Rust:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   The installer asks about settings — press Enter (the defaults). Then open a new terminal window (or run `source "$HOME/.cargo/env"`).

## Linux

1. A C compiler (Rust uses its linker), curl, git:

   | distribution | command |
   |---|---|
   | Ubuntu, Debian, Mint | `sudo apt install build-essential curl git` |
   | Fedora | `sudo dnf install gcc curl git` |
   | Arch, Manjaro | `sudo pacman -S --needed base-devel curl git` |
   | openSUSE | `sudo zypper install gcc curl git` |

2. Rust — the same way as on macOS:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

You don't need `-dev` packages (Wayland, X11, Cairo, Pango): uison loads these libraries at run time, and the program depends only on libc. You do need a desktop — GNOME, KDE, Sway, Xfce… — with Wayland or X11. On a headless server the window has nowhere to open.

## Windows

> This section is based on the documentation and has not yet been walked through live on a clean Windows install (planned: a check and screenshots of the installer).

1. Download **rustup-init.exe** from [rustup.rs](https://rustup.rs) and run it. On Windows, Rust builds programs with Microsoft's tools (MSVC), and the installer offers to install them:

   ```text
   1) Quick install via the Visual Studio Community installer
   ```

   Choose **1**. The Visual Studio installer opens with "MSVC C++ build tools" and "Windows SDK" already checked — click "Install" (about 2–3 GB). If Visual Studio with the "Desktop development with C++" workload is already installed, this step is skipped automatically.
2. Go back to the rustup window and press Enter (default installation).
3. **Git**: [git-scm.com](https://git-scm.com/download/win) or `winget install Git.Git`.
4. Open a **new** PowerShell or terminal window so that it picks up the paths.

## Editor

Any editor will do. It is most convenient with Rust hints:
- **VS Code** + the **rust-analyzer** extension — completion of DSL elements and properties, go to definition, errors right in the code;
- **RustRover** (JetBrains) — the same out of the box;
- **Zed**, **Helix** — rust-analyzer is built in.

## Checking

```sh
rustc --version
cargo --version
git --version
```

All three print a version (Rust 1.88 or newer). Now lesson 1 — straight from the uison repository:

```sh
git clone https://github.com/MarketKernel/uison
cd uison
cargo run --release -p uison --example 01-hello
```

The first build takes a minute or two, after that seconds. A window with a "+1" button opens.

![Lesson 1](../img/01-hello.png)

If something goes wrong:
- `linker 'cc' not found` (Linux) — the C compiler is not installed, step 1;
- `link.exe not found` (Windows) — the MSVC build tools are not installed: run "Visual Studio Installer" → "Modify" → "Desktop development with C++";
- the window did not open on Linux and the terminal shows a `Wayland: …` error — the program was started without a desktop (over SSH, in a container).

Next — [lesson 1: a window and a button](01-hello.md), or go straight to [your own crate](../quickstart.md).
