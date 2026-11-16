param([switch]$Force)

$ErrorActionPreference = "Stop"

# llama.cpp v1.0.0's build. A new tag needs the structs in
# crates/translator-translate/src/builtin/llama.rs checked against its llama.h.
$Tag = "b12359"
$Zip = "llama-$Tag-bin-win-vulkan-x64.zip"
$Sha256 = "e8d18c6e4210cca87e7307c2ea78363ac8d158e4e1397358ec96ab70b53a0d5e"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$OutDir = Join-Path $Root "third_party\llama.cpp"
$VersionFile = Join-Path $OutDir "VERSION"

if (-not $Force -and (Get-Content -LiteralPath $VersionFile -ErrorAction SilentlyContinue) -eq $Tag -and
    (Test-Path (Join-Path $OutDir "LICENSE*"))) {
    Write-Host "Already staged at $OutDir"
    exit 0
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$ZipPath = Join-Path $OutDir $Zip
Write-Host "==> Downloading $Zip..."
Invoke-WebRequest -Uri "https://github.com/ggml-org/llama.cpp/releases/download/$Tag/$Zip" -OutFile $ZipPath
$got = (Get-FileHash -LiteralPath $ZipPath -Algorithm SHA256).Hash
if ($got -ne $Sha256) {
    throw "$Zip SHA256 is $got, expected $Sha256"
}

# Only libllama, its backends and the licenses (OpenMP's): the zip also holds the llama-* tools and
# the RPC backend.
Get-ChildItem -LiteralPath $OutDir -File | Where-Object { $_.Name -match '\.dll$|^LICENSE' } | Remove-Item -Force
$Extract = Join-Path $OutDir "extract"
Expand-Archive -LiteralPath $ZipPath -DestinationPath $Extract -Force
Get-ChildItem -LiteralPath $Extract -Recurse -File |
    Where-Object { $_.Name -match '^(llama|ggml(-base|-vulkan|-cpu-.+)?|libomp)\.dll$|^LICENSE' } |
    Copy-Item -Destination $OutDir
Remove-Item -LiteralPath $Extract -Recurse -Force
Remove-Item -LiteralPath $ZipPath -Force

Set-Content -LiteralPath $VersionFile -Value $Tag -Encoding ascii
Write-Host "==> Staged:"
Get-ChildItem -LiteralPath $OutDir -File | Where-Object { $_.Name -match '\.dll$|^LICENSE' } | ForEach-Object { "  $($_.Name)" }
