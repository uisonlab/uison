# Lessons of the three series on Windows (docs\rust, docs\c, docs\cpp) and the assembly ones (examples-asm): build them
# and run the scenarios, like tools/lessons.sh on a Mac, but screenshots stay in out\lessons (the lesson texts show
# screenshots from the Mac).
#   tools\lessons.ps1 [all|rust|c|cpp|asm]   # asm: lessons 1 and 9 in x86-64 assembly (NASM)
#   tools\lessons.ps1 wasm      # C and C++ lessons in the browser: emcmake, headless Chrome (tools\web-test.ps1)
# C and C++ from sources (CMake + Visual Studio, cargo builds libuison); from the package: tools\c-dist.ps1 -Test.
param([ValidateSet("all", "rust", "c", "cpp", "asm", "wasm")][string]$What = "all")
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
$fail = 0

# Lesson scenario: its own scenario.json or the one shared by core lessons, scenarios\lessons\<name>.json.
function Scenario($dir, $name) {
    $own = Join-Path $dir "scenario.json"
    if (Test-Path $own) { return (Resolve-Path $own).Path }
    $shared = "scenarios\lessons\$($name.Substring(3)).json"
    if (Test-Path $shared) { return (Resolve-Path $shared).Path }
    return $null
}

function Run-Lesson($lang, $name, $exe, $scenario) {
    if (-not $scenario) { "· $lang/$name — no scenario"; return }
    $cap = "out\lessons\$lang\$name"
    if (Test-Path $cap) { Remove-Item -Recurse -Force $cap }
    $env:UISON_SCRIPT = $scenario
    $env:UISON_CAPTURE = (Join-Path (Get-Location) $cap)
    $log = & $exe 2>&1 | Out-String
    Remove-Item Env:UISON_SCRIPT, Env:UISON_CAPTURE
    $bad = $log -split "`n" | Where-Object { $_ -match "no hit area|panic|error" }
    if ($bad -or $log -notmatch "uison: done") { "✗ $lang/$name"; $bad; $script:fail = 1 }
    else { "✓ $lang/$name (screenshots: $((Get-ChildItem $cap -Filter *.png -ErrorAction SilentlyContinue).Count))" }
}

function Web-Lesson($page, $scenario, $label) {
    $name = [IO.Path]::GetFileNameWithoutExtension($page)
    & "$PSScriptRoot\web.ps1" $page | Out-Null
    Copy-Item $scenario "target\web\$name\scenarios\lesson.json"
    $log = & "$PSScriptRoot\web-test.ps1" $name lesson.json 2>&1 | Where-Object { $_ -notmatch '^capture' } | Out-String
    $shots = (Get-ChildItem "target\web\$name\capture" -Filter *.png -ErrorAction SilentlyContinue).Count
    if ($log -notmatch "DONE" -or $log -match "no hit area|panic|uison: .*(error|bad)") { "✗ ${label}:"; $log; $script:fail = 1 }
    else { "✓ $label (screenshots: $shots, target\web\$name)" }
}

if ($What -eq "wasm") {
    . "$PSScriptRoot\emsdk-env.ps1"
    foreach ($lang in "c", "cpp") {
        emcmake $cmake -S "examples-$lang" -B "out\wasm\$lang" -G Ninja -DCMAKE_BUILD_TYPE=Release *> $null
        if ($LASTEXITCODE) { exit 1 }
        $build = & $cmake --build "out\wasm\$lang" 2>&1
        if ($LASTEXITCODE -or ($build -match "warning|error")) { $build | Select-String "warning|error"; exit 1 }
        foreach ($dir in Get-ChildItem "examples-$lang\lessons" -Directory) {
            $page = "out\wasm\$lang\$lang-$($dir.Name).html"
            if (-not (Test-Path $page)) { "· $lang/$($dir.Name) — no page (threads)"; continue }
            $sc = Scenario $dir.FullName $dir.Name
            if ($sc) { Web-Lesson $page $sc "$lang/$($dir.Name)" }
        }
    }
    # Lambdas capturing main's locals by reference: main doesn't unwind in the browser.
    Web-Lesson "out\wasm\cpp\cpp-locals.html" (Resolve-Path examples-cpp\cpp-test\locals.json).Path "cpp-locals"
    exit $fail
}

if ($What -in "all", "rust") {
    cargo build -q --release -p uison --examples
    if ($LASTEXITCODE) { exit 1 }
    cargo test -q -p uison --example 09-tests
    if ($LASTEXITCODE) { $fail = 1 }
    foreach ($f in Get-ChildItem "crates\uison\examples\*.rs" | Where-Object { $_.Name -match '^\d\d-' }) {
        $name = $f.BaseName
        Run-Lesson "rust" $name "target\release\examples\$name.exe" (Scenario "crates\uison\examples\$name" $name)
    }
}

# CMake comes with Visual Studio's C++ workload; without a separate install it is not on PATH.
$cmake = (Get-Command cmake -ErrorAction SilentlyContinue).Source
if (-not $cmake) {
    $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath
    $cmake = "$vs\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe"
}

foreach ($lang in "c", "cpp", "asm") {
    if ($What -notin "all", $lang) { continue }
    $build = "out\$lang\win"
    & $cmake -S "examples-$lang" -B $build
    if ($LASTEXITCODE) { exit 1 }
    & $cmake --build $build --config Release -- /m /v:minimal
    if ($LASTEXITCODE) { exit 1 }
    foreach ($dir in Get-ChildItem "examples-$lang\lessons" -Directory) {
        $exe = "$build\Release\$lang-$($dir.Name).exe"
        if (-not (Test-Path $exe)) { "· $lang/$($dir.Name) — no program"; continue }
        Run-Lesson $lang $dir.Name $exe (Scenario $dir.FullName $dir.Name)
    }
}
exit $fail
