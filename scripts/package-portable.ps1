<#
.SYNOPSIS
  Build translator-app (release) and pack a portable ZIP (framework-dependent).

.DESCRIPTION
  Produces dist/TranslatorOverlay-<version>-win-x64.zip containing the files
  required to run a framework-dependent windows-reactor 0.100 app:

    - translator-app.exe
    - lib/onnxruntime.dll       (custom ORT with DirectML + WebGPU)
    - lib/DirectML.dll          (DirectML EP)
    - lib/webgpu_dawn.dll       (WebGPU EP / Dawn)
    - lib/dxcompiler.dll        (Dawn D3D12 shader compiler)
    - lib/dxil.dll              (DXIL validator used with dxcompiler)

  windows-reactor 0.100 inlines WASDK bootstrap (no Bootstrap.dll).
  Target machines need Windows 11 21H2 (build 22000+) and Windows App Runtime 2.4.
  See: https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads
  Visual C++ Redistributable may also be needed for MSVC CRT DLLs.

.PARAMETER SkipBuild
  Skip `cargo build --release` (use existing target/release artifacts).

.PARAMETER OutDir
  Staging / output root. Default: <repo>/dist

.PARAMETER ZipName
  Override zip file name (without path). Default: TranslatorOverlay-<ver>-win-x64.zip

.EXAMPLE
  .\scripts\package-portable.ps1

.EXAMPLE
  .\scripts\package-portable.ps1 -SkipBuild
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$OutDir = "",
    [string]$ZipName = ""
)

$ErrorActionPreference = "Stop"

function Get-RepoRoot {
    $here = $PSScriptRoot
    if (-not $here) { $here = Split-Path -Parent $MyInvocation.MyCommand.Path }
    return (Resolve-Path (Join-Path $here "..")).Path
}

function Get-AppVersion {
    param([string]$WorkspaceToml)
    $text = Get-Content -LiteralPath $WorkspaceToml -Raw
    if ($text -match '(?ms)\[workspace\.package\].*?^\s*version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    return "0.0.0"
}

$Root = Get-RepoRoot
Set-Location -LiteralPath $Root

$Version = Get-AppVersion (Join-Path $Root "Cargo.toml")
if (-not $OutDir) {
    $OutDir = Join-Path $Root "dist"
}
$StageName = "TranslatorOverlay"
$StageDir = Join-Path $OutDir $StageName
$ReleaseDir = Join-Path $Root "target\release"

if (-not $ZipName) {
    $ZipName = "TranslatorOverlay-$Version-win-x64.zip"
}
$ZipPath = Join-Path $OutDir $ZipName

Write-Host "==> Repo:    $Root"
Write-Host "==> Version: $Version"
Write-Host "==> Stage:   $StageDir"
Write-Host "==> Zip:     $ZipPath"

if (-not $SkipBuild) {
    Write-Host "==> Building release (translator-app)..."
    cargo build -p translator-app --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
} else {
    Write-Host "==> Skipping build (-SkipBuild)"
}

$ExePath = Join-Path $ReleaseDir "translator-app.exe"
if (-not (Test-Path -LiteralPath $ExePath)) {
    throw "Missing $ExePath — run without -SkipBuild or build first."
}

$SidecarDlls = @("onnxruntime.dll", "DirectML.dll", "webgpu_dawn.dll", "dxcompiler.dll", "dxil.dll")
foreach ($name in $SidecarDlls) {
    $src = Join-Path $ReleaseDir $name
    if (-not (Test-Path -LiteralPath $src)) {
        throw "Missing $name in $ReleaseDir (required by the ONNX Runtime DirectML/WebGPU build)."
    }
}

# Clean stage
if (Test-Path -LiteralPath $StageDir) {
    Remove-Item -LiteralPath $StageDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "==> Copying runtime files..."
Copy-Item -LiteralPath $ExePath -Destination (Join-Path $StageDir "translator-app.exe")
$LibDir = Join-Path $StageDir "lib"
New-Item -ItemType Directory -Force -Path $LibDir | Out-Null
foreach ($name in $SidecarDlls) {
    Copy-Item -LiteralPath (Join-Path $ReleaseDir $name) -Destination (Join-Path $LibDir $name)
}

$packReadme = @"
# Translator Overlay $Version (portable)

## Requirements
- Windows 11 x64 (21H2 / build 22000 or later)
- Windows App Runtime 2.4 (framework-dependent)
  https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads
- Microsoft Visual C++ Redistributable
- Network for first-run OCR model download (oar-ocr) and translation API

## Run
1. Extract this folder anywhere.
2. Double-click translator-app.exe
3. Configure API key in the UI and Save.

OCR DLLs live in lib/. config.toml and models/ are created next to the executable on first run.
"@
Set-Content -LiteralPath (Join-Path $StageDir "README.txt") -Value $packReadme -Encoding utf8

Write-Host "==> Staging contents:"
Get-ChildItem -LiteralPath $StageDir -Recurse -File |
    ForEach-Object {
        $rel = $_.FullName.Substring($StageDir.Length).TrimStart('\')
        $mb = [math]::Round($_.Length / 1MB, 2)
        "  $rel  ($mb MB)"
    }

if (Test-Path -LiteralPath $ZipPath) {
    Remove-Item -LiteralPath $ZipPath -Force
}

Write-Host "==> Creating ZIP..."
Compress-Archive -Path $StageDir -DestinationPath $ZipPath -CompressionLevel Optimal

$zipMb = [math]::Round((Get-Item -LiteralPath $ZipPath).Length / 1MB, 2)
Write-Host ""
Write-Host "Done."
Write-Host "  Stage: $StageDir"
Write-Host "  Zip:   $ZipPath  ($zipMb MB)"
Write-Host ""
Write-Host "Note: Users need Windows 11 and Windows App Runtime 2.4 before launching."
