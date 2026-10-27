# Run a scenario in headless Chrome, like tools/web-test.sh: tools\web-test.ps1 <package> <scenario.json> [width height]
# Screenshots go to target\web\<package>\capture\, page logs to stdout. tools\serve.py needs Python: the one on PATH or
# emsdk's own (~\emscripten\emsdk\python).
param(
    [Parameter(Mandatory)][string]$Pkg,
    [Parameter(Mandatory)][string]$Script,
    [int]$W = 1220,
    [int]$H = 780,
    [int]$Port = 8766
)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
$dir = "target\web\$Pkg"
if (Test-Path "$dir\capture") { Remove-Item -Recurse -Force "$dir\capture" }

# The Microsoft Store alias "python" is not Python: skip WindowsApps.
$python = Get-Command python, python3 -ErrorAction SilentlyContinue | Where-Object { $_.Source -notmatch 'WindowsApps' } | Select-Object -First 1 -ExpandProperty Source
if (-not $python) {
    $python = Get-ChildItem "$HOME\emscripten\emsdk\python\*\python.exe" -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $python) { Write-Error "no Python for tools\serve.py (install emsdk or Python)" }
$chrome = @("$env:ProgramFiles\Google\Chrome\Application\chrome.exe", "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
    "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $chrome) { Write-Error "no Chrome" }

$log = New-TemporaryFile
$profile = Join-Path ([IO.Path]::GetTempPath()) "uison-chrome-$PID"
$env:PYTHONUTF8 = "1"
$server = Start-Process -FilePath $python -ArgumentList "-u", "tools\serve.py", $dir, $Port -RedirectStandardOutput $log -RedirectStandardError "$log.err" -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 700
$url = "http://localhost:$Port/?test=1&stats=$(if ($env:STATS) { $env:STATS } else { 0 })$($env:URL_EXTRA)&script=scenarios/$Script"
$browser = Start-Process -FilePath $chrome -PassThru -ArgumentList "--headless=new", "--user-data-dir=$profile", "--no-first-run",
    "--hide-scrollbars", "--force-device-scale-factor=2", "--window-size=$W,$H", $url
try {
    for ($i = 0; $i -lt 120; $i++) {
        if (Select-String -Path $log -Pattern DONE -Quiet) { break }
        Start-Sleep -Milliseconds 250
    }
} finally {
    # Chrome starts helper processes: end the whole tree.
    taskkill /PID $browser.Id /T /F 2>&1 | Out-Null
    Stop-Process $server -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
    Remove-Item -Recurse -Force $profile -ErrorAction SilentlyContinue
}
Get-Content $log | Where-Object { $_ -notmatch '^uison: http' }
Remove-Item $log, "$log.err" -ErrorAction SilentlyContinue
