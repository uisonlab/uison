# Dot-source it: . tools\emsdk-env.ps1 — emcc/em++/emcmake on PATH (emsdk from $env:EMSDK, ~\emscripten\emsdk or ~\emsdk),
# $cmake (on PATH or Visual Studio's own) and Ninja for it (emcmake needs a Makefile or Ninja generator; Visual Studio has Ninja).
if (-not (Get-Command emcc -ErrorAction SilentlyContinue)) {
    $emsdk = @($env:EMSDK, "$HOME\emscripten\emsdk", "$HOME\emsdk") | Where-Object { $_ -and (Test-Path "$_\emsdk_env.ps1") } | Select-Object -First 1
    if (-not $emsdk) { Write-Error "no emcc: install emsdk (https://emscripten.org/docs/getting_started/downloads.html)" }
    . "$emsdk\emsdk_env.ps1" *> $null
    # emsdk's scripts turn strict mode on, and dot-sourcing leaves it on for the caller.
    Set-StrictMode -Off
}
$cmake = (Get-Command cmake -ErrorAction SilentlyContinue).Source
if (-not $cmake -or -not (Get-Command ninja -ErrorAction SilentlyContinue)) {
    $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath
    $vsCMake = "$vs\Common7\IDE\CommonExtensions\Microsoft\CMake"
    if (-not $cmake) { $cmake = "$vsCMake\CMake\bin\cmake.exe" }
    if (-not (Get-Command ninja -ErrorAction SilentlyContinue)) { $env:PATH = "$vsCMake\Ninja;$env:PATH" }
}
