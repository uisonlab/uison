# Build and run an example on Windows: scenario, window screenshots, hit area map, log.
#
#   tools\win.ps1 studio                         # window until closed
#   tools\win.ps1 studio studio.json             # scenario (scenarios\ or web\scenarios\), screenshots in out\win\studio
#   tools\win.ps1 studio -Seconds 5 -Stats       # 5 seconds with UISON_STATS, log in out\win\studio\log.txt
#   tools\win.ps1 todo -Dev -Flash               # debug build with a console, redraw highlighting
#   tools\win.ps1 studio -Arch arm64             # Windows on ARM: on x64 build only (needs MSVC ARM64 build tools)
#   tools\win.ps1 -CheckVtables                  # COM method indices from the SDK headers (needs MSVC)
#
# While the example runs, it can be driven with the real mouse:
#   target\windrive.exe studio map out\win\studio\hitmap.txt tap inboxList/2
# The script is UTF-8 with LF: run it from pwsh 7 (Windows PowerShell 5.1 breaks non-ASCII strings).

param(
    [Parameter(Position = 0)][string]$Example = "studio",
    [Parameter(Position = 1)][string]$Script = "",
    [switch]$Dev,
    [int]$Seconds = 0,
    [switch]$Stats,
    [switch]$Flash,
    [switch]$Timer,
    [string]$Out = "",
    [switch]$CheckVtables,
    [ValidateSet("", "x64", "arm64")][string]$Arch = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# windrive is the real-input tool; rebuild it if the source is newer.
$drive = "target\windrive.exe"
if (-not (Test-Path $drive) -or (Get-Item "tools\windrive.rs").LastWriteTime -gt (Get-Item $drive).LastWriteTime) {
    New-Item -ItemType Directory -Force target | Out-Null
    rustc -O --edition 2024 -o $drive tools\windrive.rs
    if ($LASTEXITCODE) { exit 1 }
}

if ($CheckVtables) {
    $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath
    $vcvars = Join-Path $vs "VC\Auxiliary\Build\vcvars64.bat"
    cmd /c "`"$vcvars`" >nul && cl /nologo /EHsc /std:c++17 /O1 tools\vtable.cpp /Fe:target\vtable.exe /Fo:target\ >nul"
    if ($LASTEXITCODE) { exit 1 }
    # The indices uison-windows relies on are in tools\vtable.txt: a mismatch is an error.
    $now = @(& target\vtable.exe)
    $diff = Compare-Object (Get-Content tools\vtable.txt) $now
    if ($diff) { $diff; Write-Error "COM method indices differ from tools\vtable.txt" }
    "vtable: $($now.Count) methods match tools\vtable.txt"
    exit 0
}

$build = if ($Dev) { "debug" } else { "release" }
$cargoArgs = @("build", "-p", $Example)
if (-not $Dev) { $cargoArgs += "--release" }
$hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
if (-not $Arch) { $Arch = $hostArch }
$dir = "target\$build"
if ($Arch -ne $hostArch) {
    # Cross build: cargo looks for the linker only in the newest MSVC toolset, while the cross toolset
    # may be in another one (VS 18: 14.51 without ARM64, ARM64 build tools in 14.44). Take the newest
    # that has a linker for the target architecture, plus its libraries and the Windows SDK libraries.
    $triple = if ($Arch -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
    $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath
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
    $cargoArgs += @("--target", $triple)
    $dir = "target\$triple\$build"
}
cargo @cargoArgs
if ($LASTEXITCODE) { exit 1 }
$exe = "$dir\$Example.exe"
if ($Arch -ne $hostArch -and $Arch -eq "arm64") {
    # x64 can't run ARM64 (the reverse works: x64 emulation on Windows on ARM).
    "built $exe ($Arch); run it on a Windows on ARM machine"
    exit 0
}

if (-not $Out) { $Out = "out\win\$Example" }
Remove-Item -Recurse -Force $Out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path

$env:UISON_CAPTURE = if ($Script) { $Out } else { "" }
$env:UISON_HITMAP = "$Out\hitmap.txt"
$env:UISON_STATS = if ($Stats) { "1" } else { "" }
$env:UISON_FLASH = if ($Flash) { "1" } else { "" }
$env:UISON_TIMER = if ($Timer) { "1" } else { "" }
$env:UISON_SCRIPT = ""
if ($Script) {
    $found = @($Script, "scenarios\$Script", "web\scenarios\$Script") | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $found) { Write-Error "no scenario $Script" }
    $env:UISON_SCRIPT = (Resolve-Path $found).Path
}

$log = "$Out\log.txt"
$p = Start-Process -FilePath $exe -RedirectStandardError $log -PassThru
if ($Script) {
    # The scenario ends with a done step, and the app exits by itself.
    if (-not $p.WaitForExit(180000)) { Stop-Process $p; Write-Warning "scenario timed out" }
} elseif ($Seconds -gt 0) {
    Start-Sleep -Seconds $Seconds
    Stop-Process $p -ErrorAction SilentlyContinue
} else {
    $p.WaitForExit()
}
Get-Content $log
Get-ChildItem $Out -Filter *.png | ForEach-Object { $_.FullName }
