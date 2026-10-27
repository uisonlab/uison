# Build a uison app for the browser, like tools/web.sh: tools\web.ps1 <package | program.html>
# A package (Rust, wasm32-unknown-unknown): target\web\<package>\ — app.wasm + the shared index.html, uison.js, drive.js.
# A built C/C++ program (Emscripten, an emcc page): tools\web.ps1 out\wasm\cpp\todo-cpp.html — target\web\todo-cpp\
# gets the page (index.html), its .js/.wasm/.data and the files next to it (images, JSON).
# wasm-opt (binaryen, it comes with emsdk) is used when it is on PATH; NO_WASM_OPT=1 skips it.
# Run: python tools\serve.py target\web\<name>  →  http://localhost:8000/
param([Parameter(Mandatory)][string]$Pkg)
$ErrorActionPreference = "Stop"
if ($Pkg -match '\.html$') { $Pkg = (Resolve-Path $Pkg).Path }
Set-Location (Join-Path $PSScriptRoot "..")

function Size($f) {
    $bytes = [IO.File]::ReadAllBytes($f)
    $ms = [IO.MemoryStream]::new()
    $gz = [IO.Compression.GZipStream]::new($ms, [IO.Compression.CompressionLevel]::SmallestSize)
    $gz.Write($bytes, 0, $bytes.Length); $gz.Close()
    "$($bytes.Length) B (gzip $($ms.ToArray().Length))"
}

function Copy-Scenarios($out) {
    New-Item -ItemType Directory -Force "$out\scenarios" | Out-Null
    # Shared scenarios (uison_frame::script): drive.js understands the same steps; web-specific ones take precedence.
    Copy-Item scenarios\*.json "$out\scenarios\"
    Copy-Item web\scenarios\*.json "$out\scenarios\"
}

if ($Pkg -match '\.html$') {
    $src = Split-Path $Pkg
    $name = [IO.Path]::GetFileNameWithoutExtension($Pkg)
    $out = "target\web\$name"
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force $out | Out-Null
    Copy-Item $Pkg "$out\index.html"
    foreach ($ext in "js", "wasm", "data") {
        if (Test-Path "$src\$name.$ext") { Copy-Item "$src\$name.$ext" "$out\" }
    }
    Get-ChildItem $src -File | Where-Object { $_.Extension -in ".jpg", ".png", ".json" } | Copy-Item -Destination "$out\"
    Copy-Item web\drive.js "$out\"
    Copy-Scenarios $out
    "${out}: wasm $(Size "$out\$name.wasm"), js $(Size "$out\$name.js")"
    exit 0
}

cargo build --release --target wasm32-unknown-unknown -p $Pkg
if ($LASTEXITCODE) { exit 1 }
$out = "target\web\$Pkg"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item "target\wasm32-unknown-unknown\release\$Pkg.wasm" "$out\app.wasm"
if (-not $env:NO_WASM_OPT -and (Get-Command wasm-opt -ErrorAction SilentlyContinue)) {
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals -o "$out\app.wasm" "$out\app.wasm"
}
Copy-Item web\index.html, web\uison.js, web\drive.js "$out\"
Copy-Scenarios $out
"$out\app.wasm: $(Size "$out\app.wasm")"
