# Compile the Vulkan renderer's shaders (crates/uison-vulkan-windows/shaders/*.vert, *.frag) to SPIR-V.
# The .spv files are in the repository: building uison needs no Vulkan SDK, only changing a shader does.
#   tools/vulkan-shaders.ps1
$ErrorActionPreference = 'Stop'
if (-not $env:VULKAN_SDK) { throw 'VULKAN_SDK is not set: install the Vulkan SDK (glslc comes with it)' }
$glslc = Join-Path $env:VULKAN_SDK 'Bin\glslc.exe'
$dir = Join-Path $PSScriptRoot '..\crates\uison-vulkan-windows\shaders'
foreach ($src in Get-ChildItem $dir -Include *.vert, *.frag -Name) {
    & $glslc -O --target-env=vulkan1.0 (Join-Path $dir $src) -o (Join-Path $dir "$src.spv")
    if ($LASTEXITCODE) { throw "glslc failed on $src" }
    Write-Output "$src.spv"
}
