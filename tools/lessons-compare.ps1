# Core lessons in the three languages are the same program: Rust, C and C++ screenshots must match pixel for pixel
# (like tools/lessons-compare.sh). Windows captures the client area — no title bar to skip.
#   tools\lessons-compare.ps1        # lessons in windows (run tools\lessons.ps1 first)
#   tools\lessons-compare.ps1 web    # examples in the browser: Rust (tools\web.ps1) against C/C++ (emcmake)
param([ValidateSet("", "web")][string]$What = "")
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
Add-Type -AssemblyName System.Drawing
# PNGs are decoded by System.Drawing here; the C# part sees only bytes (in pwsh 7 Bitmap lives in assemblies
# Add-Type can't reference simply).
Add-Type -TypeDefinition @"
public static class PixelDiff {
    // Bounding box of the differing pixels as "x0,y0,x1,y1"; null if equal. 32-bit pixels, rows of `stride` bytes.
    public static string Compare(byte[] a, byte[] b, int stride) {
        int x0 = int.MaxValue, y0 = int.MaxValue, x1 = -1, y1 = -1;
        for (int i = 0; i < a.Length; i += 4) {
            if (a[i] == b[i] && a[i + 1] == b[i + 1] && a[i + 2] == b[i + 2] && a[i + 3] == b[i + 3]) continue;
            int row = i / stride, col = (i % stride) / 4;
            if (col < x0) x0 = col; if (row < y0) y0 = row; if (col > x1) x1 = col; if (row > y1) y1 = row;
        }
        return x1 < 0 ? null : x0 + "," + y0 + "," + x1 + "," + y1;
    }
}
"@

function Pixels($file) {
    $bmp = [System.Drawing.Bitmap]::new((Resolve-Path $file).Path)
    try {
        $r = [System.Drawing.Rectangle]::new(0, 0, $bmp.Width, $bmp.Height)
        $d = $bmp.LockBits($r, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $bytes = [byte[]]::new($d.Stride * $bmp.Height)
        [System.Runtime.InteropServices.Marshal]::Copy($d.Scan0, $bytes, 0, $bytes.Length)
        $bmp.UnlockBits($d)
        @{ Size = "$($bmp.Width)x$($bmp.Height)"; Stride = $d.Stride; Bytes = $bytes }
    } finally { $bmp.Dispose() }
}

# "x0,y0,x1,y1" of the difference, "size", or $null if the images are equal.
function Diff-Png($a, $b) {
    $pa = Pixels $a; $pb = Pixels $b
    if ($pa.Size -ne $pb.Size) { return "size" }
    [PixelDiff]::Compare($pa.Bytes, $pb.Bytes, $pa.Stride)
}
$fail = 0

# The first folder is the reference, the others are checked against it.
function Compare-Shots([string[]]$dirs) {
    $shots = Get-ChildItem $dirs[0] -Filter *.png | ForEach-Object Name
    if (-not $shots) { "  $($dirs[0]): no screenshots"; $script:fail = 1; return }
    foreach ($d in $dirs[1..($dirs.Count - 1)]) {
        foreach ($n in $shots) {
            if (-not (Test-Path "$d\$n")) { "  ${d}: no screenshot $n"; $script:fail = 1; continue }
            $diff = Diff-Png "$($dirs[0])\$n" "$d\$n"
            if ($diff) { "  $d\${n}: DIFFERENT ($diff)"; $script:fail = 1 } else { "  $d\${n}: match" }
        }
    }
}

if ($What -eq "web") {
    . "$PSScriptRoot\emsdk-env.ps1"
    foreach ($lang in "c", "cpp") {
        emcmake $cmake -S "examples-$lang" -B "out\wasm\$lang" -G Ninja -DCMAKE_BUILD_TYPE=Release *> $null
        if ($LASTEXITCODE) { exit 1 }
    }
    & $cmake --build out\wasm\c --target form2-c *> $null
    & $cmake --build out\wasm\cpp --target todo-cpp list10k-cpp *> $null
    foreach ($p in @(@("form2", "c\form2-c", "form2.json"), @("todo", "cpp\todo-cpp", "todo.json"), @("list10k", "cpp\list10k-cpp", "list10k.json"))) {
        $rs, $c, $sc = $p
        $cname = Split-Path $c -Leaf
        & "$PSScriptRoot\web.ps1" $rs *> $null
        & "$PSScriptRoot\web.ps1" "out\wasm\$c.html" *> $null
        foreach ($pkg in $rs, $cname) {
            $log = & "$PSScriptRoot\web-test.ps1" $pkg $sc 2>&1 | Out-String
            if ($log -match "no hit area|error" -or $log -notmatch "DONE") { "✗ ${pkg}:"; $log; $fail = 1 }
        }
        "${cname}:"
        Compare-Shots "target\web\$rs\capture", "target\web\$cname\capture"
    }
    exit $fail
}

# Core lessons: the same name after the number (01-hello, 06-themes / 07-themes…) means the same program.
foreach ($name in "hello", "data", "forms", "lists", "themes", "rich", "threads", "frame") {
    $dirs = @(foreach ($lang in "rust", "c", "cpp", "asm") { Get-ChildItem "out\lessons\$lang" -Directory -Filter "??-$name" -ErrorAction SilentlyContinue | ForEach-Object FullName })
    if ($dirs.Count -lt 2) { "· ${name}: no screenshots (tools\lessons.ps1)"; continue }
    "${name}:"
    Compare-Shots $dirs
}
exit $fail
