# Lesson 0. Setting up your system

**English** · [Русский](../../ru/cpp/lessons/00-setup.md)

**What you'll get:** a C++20 compiler, CMake and the uison library on your machine; at the end, lesson 1.

You need:
- **a C++20 compiler** — Apple clang from Xcode 15 or newer (macOS), GCC 11+ or clang 14+ (Linux), MSVC from Visual Studio 2022 or 2026 (Windows). The widgets (`uison_widgets.hpp`) are C++20: designated initializers; the core (`uison.hpp`) also works with C++17 ([lesson 11](11-cpp17.md));
- **CMake** 3.19 or newer — builds the lessons and your program;
- **the uison library** — either **Rust** builds it from source (CMake runs it for you), or you download a **prebuilt package**, and then you don't need Rust;
- **git** — to get the source.

You don't need to install any libraries for windows, graphics or fonts: uison takes them from the system.

## macOS

1. The compiler and git come with Apple's command line tools:

   ```sh
   xcode-select --install
   ```

   If you have Xcode installed, you already have them. You only need the full Xcode for the iPhone ([lesson 12](12-build.md)).
2. CMake: `brew install cmake` ([Homebrew](https://brew.sh)) or the `.dmg` from [cmake.org](https://cmake.org/download/) (then, in the terminal, run `sudo "/Applications/CMake.app/Contents/bin/cmake-gui" --install`).
3. Rust (builds the library):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   Press Enter for the default installation, then open a new terminal window.

## Linux

| distribution | command |
|---|---|
| Ubuntu 22.04+, Debian 12+, Mint | `sudo apt install build-essential cmake curl git` (GCC 11+) |
| Fedora | `sudo dnf install gcc cmake curl git` |
| Arch, Manjaro | `sudo pacman -S --needed base-devel cmake curl git` |
| openSUSE | `sudo zypper install gcc cmake curl git` |

Then Rust — the same way as on macOS. You don't need the `-dev` packages (Wayland, X11, Cairo, Pango): uison loads these libraries at run time, and the program depends only on libc. You need a desktop session with Wayland or X11.

## Windows

> The "Visual Studio" part was checked live with Visual Studio 2026 (Open Folder, startup item, debugging — screenshots below); the installation itself has not yet been walked through on a clean Windows.

1. **Visual Studio Community** (free; 2026, 2022 works too): [visualstudio.microsoft.com](https://visualstudio.microsoft.com/downloads/). In the installer, on the "Workloads" tab, check **"Desktop development with C++"**. It includes the MSVC compiler, the Windows SDK and CMake — you don't need to install them separately. About 7 GB.
2. **Git**: [git-scm.com](https://git-scm.com/download/win) or `winget install Git.Git`.
3. **Rust**: [rustup-init.exe](https://rustup.rs) — press Enter (default installation). Visual Studio is already installed, so the installer finds MSVC right away.
4. Open a new terminal window so the new paths are picked up. The `cl` and `cmake` commands are available in the **"Developer Command Prompt for VS"** (Start menu; for VS 2022 the name ends with "2022"), and in Visual Studio right away.

### Visual Studio

Visual Studio opens a CMake project without a `.sln`:

1. **File → Open → Folder…** — the folder with `CMakeLists.txt` (for the lessons, `examples-cpp` from the uison repository). Visual Studio runs CMake by itself; the first time, CMake calls cargo and builds the library — a minute or two, progress is shown in the "Output" window.

   ![Open Folder](../img/setup-open-folder.png)

2. On the toolbar, pick the configuration (**x64-Debug** is created by itself; others live in `CMakeSettings.json` or `CMakePresets.json`) and **"Select Startup Item"** — the program, for example `cpp-01-hello.exe` (at first Visual Studio picks the first program of the list by itself).

   ![Startup item](../img/setup-startup-item.png)

3. **F5** builds and runs under the debugger, **Ctrl+F5** without it. A breakpoint in the button's lambda (lesson 1) is hit when you press the button: you can see `cx`, the data, the stack. IntelliSense suggests widget fields with their descriptions.

   ![Breakpoint](../img/setup-breakpoint.png)

4. The program ends up in `out\build\x64-Debug\` inside the project folder.
5. Non-ASCII text in strings: uison sources are UTF-8, and the compiler needs the `/utf-8` switch (the lessons already have it in `CMakeLists.txt`).

The other way, without the Visual Studio IDE: the "Developer Command Prompt" and a single `cl` command ([lesson 12](12-build.md#windows-a-single-command)).

## A prebuilt package instead of Rust

If you'd rather not install Rust: the repository releases (`nightly` or a version) contain the packages `uison-macos`, `uison-linux-x86_64`, `uison-windows-x64`… Unpack one and give CMake its path:

```sh
cmake -S examples-cpp -B build -DUISON_PACKAGE=/path/to/uison-macos
```

## Editor

- **Visual Studio** (Windows) — see above.
- **VS Code** + the **C/C++** (or **clangd**) and **CMake Tools** extensions: "CMake: Select Kit" (the compiler), "CMake: Build", F5.
- **CLion** — opens `CMakeLists.txt` as a project.

## Checking

```sh
c++ --version       # Windows: cl (in the "Developer Command Prompt")
cmake --version     # 3.19 or newer
rustc --version     # not needed if you took a prebuilt package
git --version
```

Lesson 1 from the uison repository:

```sh
git clone https://github.com/MarketKernel/uison
cd uison
cmake -S examples-cpp -B out/cpp/build -DCMAKE_BUILD_TYPE=Release
cmake --build out/cpp/build --target cpp-01-hello --config Release
./out/cpp/build/cpp-01-hello      # Windows: out\cpp\build\Release\cpp-01-hello.exe
```

A window with a "+1" button opens.

![Lesson 1](../img/01-hello.png)

If something goes wrong:
- `Could not find UISON_CARGO` — Rust is not installed, or the terminal window was opened before the installation; alternatively, take a prebuilt package (`-DUISON_PACKAGE=…`);
- `No CMAKE_CXX_COMPILER could be found` — there is no compiler: on macOS run `xcode-select --install`, on Linux install `build-essential` / `gcc`, on Windows the "Desktop development with C++" workload;
- on Linux the window doesn't open and the terminal shows a `Wayland: …` error — the program was started without a desktop session (over SSH, in a container).

Next: [lesson 1: a window and a button](01-hello.md).
