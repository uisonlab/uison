# Library package for C/C++ on Windows: dist\uison-windows-<arch>\
#   include\uison.h, uison.hpp, uison_widgets.hpp · lib\uison.lib (static), uison.dll.lib (import) · bin\uison.dll · lib\cmake\uison (find_package)
#
#   tools\c-dist.ps1                 # x64 (or arm64 on Windows on ARM)
#   tools\c-dist.ps1 -Arch arm64     # Windows on ARM from x64 (needs MSVC ARM64 build tools)
#   tools\c-dist.ps1 -Test           # + build examples-c, examples-cpp (and examples-asm with NASM) from the package (CMake),
#                                    #   tests and lesson scenarios
#   tools\c-dist.ps1 -Test -NoLessons  # headless tests only (abi-test, cpp-test) — for CI
#   tools\c-dist.ps1 -Wasm [-Test]   # dist\uison-wasm for Emscripten (emsdk: $env:EMSDK, ~\emscripten\emsdk or ~\emsdk);
#                                    # -Test: em++ of lesson 1, emcmake + find_package, ABI tests in node (as in CI)
#
# One uison.lib for /MT and /MD: it has no CRT directives (std is built once per target,
# +crt-static only changes rustc's hint "/defaultlib:libcmt"); the program chooses the CRT.
# uison.dll has vcruntime inside and uses the system UCRT: no Visual C++ Redistributable needed.
# The script is UTF-8 with LF: run it from pwsh 7.

param(
    [ValidateSet("", "x64", "arm64")][string]$Arch = "",
    [switch]$Test,
    [switch]$NoLessons,
    [switch]$Wasm
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if ($Wasm) {
    . "$PSScriptRoot\emsdk-env.ps1"
    cargo rustc --release -q -p uison-c --target wasm32-unknown-emscripten --crate-type staticlib
    if ($LASTEXITCODE) { exit 1 }
    $out = "dist\uison-wasm"
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force "$out\include", "$out\lib\cmake\uison", "$out\bin", "$out\web" | Out-Null
    Copy-Item crates\uison-c\include\uison.h, crates\uison-c\include\uison_kit.h, crates\uison-c\include\uison.hpp, crates\uison-c\include\uison_widgets.hpp "$out\include\"
    Copy-Item crates\uison-c\cmake\uisonConfig.cmake, crates\uison-c\cmake\uison-libs.cmake "$out\lib\cmake\uison\"
    Copy-Item target\wasm32-unknown-emscripten\release\libuison.a "$out\lib\"
    Copy-Item web\shell.html "$out\web\"
    # uison-gen.exe is for the machine that builds the page.
    cargo build --release -q -p uison-gen
    Copy-Item target\release\uison-gen.exe "$out\bin\"
    if (Test-Path LICENSE) { Copy-Item LICENSE "$out\" }
    Copy-Item docs\package-readme.md "$out\README.md"
    "${out}: libuison.a $([int]((Get-Item "$out\lib\libuison.a").Length / 1024)) KB (wasm32, Emscripten)"
    if (-not $Test) { exit 0 }

    $pkg = (Resolve-Path $out).Path
    $fail = 0
    New-Item -ItemType Directory -Force out\wasm\hello | Out-Null
    em++ -std=c++20 -Oz -fwasm-exceptions "-I$pkg\include" examples-cpp\lessons\01-hello\main.cpp "$pkg\lib\libuison.a" `
        --shell-file "$pkg\web\shell.html" -o out\wasm\hello\index.html
    if ($LASTEXITCODE) { exit 1 }
    Get-ChildItem out\wasm\hello | ForEach-Object { "  $($_.Name) $($_.Length) B" }
    foreach ($lang in "c", "cpp") {
        emcmake $cmake -S "examples-$lang" -B "out\wasm\pkg-$lang" -G Ninja -DCMAKE_BUILD_TYPE=Release "-DUISON_PACKAGE=$pkg" *> $null
        if ($LASTEXITCODE) { exit 1 }
        & $cmake --build "out\wasm\pkg-$lang"
        if ($LASTEXITCODE) { exit 1 }
    }
    # From the test's own folder, as in CI.
    Push-Location examples-c\abi-test
    foreach ($t in "abi-test", "kit-test") { node "$root\out\wasm\pkg-c\$t.js"; if ($LASTEXITCODE) { "FAILED: $t"; $fail = 1 } }
    Pop-Location
    Push-Location examples-cpp\cpp-test
    node "$root\out\wasm\pkg-cpp\cpp-test.js"; if ($LASTEXITCODE) { "FAILED: cpp-test"; $fail = 1 }
    Pop-Location
    exit $fail
}

$hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
if (-not $Arch) { $Arch = $hostArch }
$triple = if ($Arch -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath

if ($Arch -ne $hostArch) {
    # Cross build: the linker for the target architecture comes from the newest toolset that has it (as in win.ps1).
    $hostDir = if ($hostArch -eq "arm64") { "Hostarm64" } else { "Hostx64" }
    $msvc = Get-ChildItem "$vs\VC\Tools\MSVC" -Directory | Where-Object { $_.Name -match '^\d+(\.\d+)+$' } | Sort-Object { [version]$_.Name } -Descending |
        Where-Object { Test-Path "$($_.FullName)\bin\$hostDir\$Arch\link.exe" } | Select-Object -First 1
    if (-not $msvc) { Write-Error "no MSVC toolset with $hostDir\$Arch\link.exe: install MSVC $Arch build tools" }
    # Version folders only: others sit next to them (wdf on GitHub runners).
    $kits = "${env:ProgramFiles(x86)}\Windows Kits\10\Lib"
    $sdk = Get-ChildItem $kits -Directory | Where-Object { $_.Name -match '^\d+(\.\d+)+$' } | Sort-Object { [version]$_.Name } -Descending |
        Where-Object { Test-Path "$($_.FullName)\um\$Arch\user32.lib" } | Select-Object -First 1
    $env:LIB = "$($msvc.FullName)\lib\$Arch;$($sdk.FullName)\um\$Arch;$($sdk.FullName)\ucrt\$Arch"
    [Environment]::SetEnvironmentVariable("CARGO_TARGET_$($triple.ToUpper().Replace('-', '_'))_LINKER", "$($msvc.FullName)\bin\$hostDir\$Arch\link.exe")
}

cargo build --release -p uison-c --target $triple
if ($LASTEXITCODE) { exit 1 }
$src = "target\$triple\release"

$out = "dist\uison-windows-$Arch"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force "$out\include", "$out\lib\cmake\uison", "$out\bin" | Out-Null
Copy-Item crates\uison-c\include\uison.h, crates\uison-c\include\uison_kit.h, crates\uison-c\include\uison.hpp, crates\uison-c\include\uison_widgets.hpp "$out\include\"
Copy-Item crates\uison-c\cmake\uisonConfig.cmake, crates\uison-c\cmake\uison-libs.cmake "$out\lib\cmake\uison\"
Copy-Item "$src\uison.lib", "$src\uison.dll.lib" "$out\lib\"
Copy-Item "$src\uison.dll" "$out\bin\"
# uison-gen.exe — widgets for custom components (uison_generate_widgets): for the build machine.
cargo build --release -q -p uison-gen
Copy-Item target\release\uison-gen.exe "$out\bin\"
if (Test-Path LICENSE) { Copy-Item LICENSE "$out\" }
Copy-Item docs\package-readme.md "$out\README.md"

$kb = { param($f) [int]((Get-Item $f).Length / 1024) }
"${out}: uison.lib $(& $kb "$out\lib\uison.lib") KB, uison.dll $(& $kb "$out\bin\uison.dll") KB"

if (-not $Test) { exit 0 }
if ($Arch -ne $hostArch -and $Arch -eq "arm64") { "skip -Test: x64 cannot run ARM64"; exit 0 }

# ---------------------------------------------------------------- package check
# examples-c and examples-cpp from the package: Visual Studio generator, CMake's default CRT (/MD) — checks
# that uison.lib links with it; tests run headless, lessons with a scenario (UISON_SCRIPT), as in tools/lessons.sh.
$cmakeArch = if ($Arch -eq "arm64") { "ARM64" } else { "x64" }
# CMake comes with Visual Studio's C++ workload; without a separate install it is not on PATH.
$cmake = (Get-Command cmake -ErrorAction SilentlyContinue).Source
if (-not $cmake) { $cmake = "$vs\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe" }
$fail = 0
# examples-asm (x86-64 only) — when NASM is installed: on PATH or where CMake looks for it besides.
$langs = @("c", "cpp")
$nasm = (Get-Command nasm -ErrorAction SilentlyContinue) -or (Test-Path "$env:ProgramFiles\NASM\nasm.exe") -or
    (Test-Path "$env:LOCALAPPDATA\NASM\nasm.exe")
if ($Arch -eq "x64" -and $nasm) { $langs += "asm" } elseif ($Arch -eq "x64") { "· examples-asm skipped: no NASM" }
foreach ($lang in $langs) {
    $build = "out\$lang\win-$Arch"
    & $cmake -S "examples-$lang" -B $build -A $cmakeArch "-DUISON_PACKAGE=$((Resolve-Path $out).Path)"
    if ($LASTEXITCODE) { exit 1 }
    & $cmake --build $build --config Release -- /m /v:minimal
    if ($LASTEXITCODE) { exit 1 }
    $bin = "$build\Release"

    $tests = switch ($lang) { "c" { @("abi-test", "kit-test", "kit-test-c11") } "cpp" { @("cpp-test", "cpp-test-shared") } default { @() } }
    foreach ($t in $tests) {
        & "$bin\$t.exe"
        if ($LASTEXITCODE) { "FAILED: $t"; $fail = 1 }
    }
    if ($NoLessons) { continue }
    # Lessons (c-01-hello.exe, cpp-02-data.exe…): the scenario is the lesson's own scenario.json or the one shared by
    # core lessons, scenarios\lessons\<name>.json — as in tools/lessons.sh.
    foreach ($dir in Get-ChildItem "examples-$lang\lessons" -Directory) {
        $scenario = Join-Path $dir.FullName "scenario.json"
        if (-not (Test-Path $scenario)) { $scenario = Join-Path (Get-Location) "scenarios\lessons\$($dir.Name.Substring(3)).json" }
        $exe = "$bin\$lang-$($dir.Name).exe"
        if (-not (Test-Path $scenario) -or -not (Test-Path $exe)) { continue }
        $env:UISON_SCRIPT = $scenario
        $env:UISON_CAPTURE = "$build\capture\$lang-$($dir.Name)"
        $log = & $exe 2>&1 | Out-String
        Remove-Item Env:UISON_SCRIPT, Env:UISON_CAPTURE
        $bad = $log -split "`n" | Where-Object { $_ -match "no hit area|panic|error" }
        if ($bad -or $log -notmatch "uison: done") { "FAILED: $lang-$($dir.Name)"; $bad; $fail = 1 }
        else { "ok: $lang-$($dir.Name)" }
    }
}
exit $fail
