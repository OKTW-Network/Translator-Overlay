<#
.SYNOPSIS
  Build translator-app (release) and pack a portable ZIP (framework-dependent).

.DESCRIPTION
  Produces dist/TranslatorOverlay-<version>-win-x64.zip containing only the files
  required to run a framework-dependent windows-reactor app:

    - translator-app.exe
    - Microsoft.WindowsAppRuntime.Bootstrap.dll
    - DirectML.dll          (ONNX Runtime GPU EP; PE import — required next to exe)
    - resources.pri

  Target machines must have a matching Windows App Runtime installed.
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
    param([string]$CargoToml)
    $text = Get-Content -LiteralPath $CargoToml -Raw
    if ($text -match '(?m)^\s*version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    return "0.0.0"
}

$Root = Get-RepoRoot
Set-Location -LiteralPath $Root

$Version = Get-AppVersion (Join-Path $Root "crates\translator-app\Cargo.toml")
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

$BootstrapPath = Join-Path $ReleaseDir "Microsoft.WindowsAppRuntime.Bootstrap.dll"
if (-not (Test-Path -LiteralPath $BootstrapPath)) {
    throw @"
Missing Microsoft.WindowsAppRuntime.Bootstrap.dll in $ReleaseDir.
Ensure build.rs calls windows_reactor_setup::as_framework_dependent() and rebuild.
"@
}

$DirectMlPath = Join-Path $ReleaseDir "DirectML.dll"
if (-not (Test-Path -LiteralPath $DirectMlPath)) {
    throw @"
Missing DirectML.dll in $ReleaseDir (required PE dependency for oar-ocr / DirectML).
Rebuild: cargo build -p translator-app --release
ort-sys should copy DirectML.dll next to the exe when the `directml` feature is on.
"@
}

$ResourcesPri = Join-Path $ReleaseDir "resources.pri"
if (-not (Test-Path -LiteralPath $ResourcesPri)) {
    Write-Warning "resources.pri not found in release dir (continuing; app may still run)."
}

# Clean stage
if (Test-Path -LiteralPath $StageDir) {
    Remove-Item -LiteralPath $StageDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "==> Copying runtime files..."
Copy-Item -LiteralPath $ExePath -Destination (Join-Path $StageDir "translator-app.exe")
Copy-Item -LiteralPath $BootstrapPath -Destination (Join-Path $StageDir "Microsoft.WindowsAppRuntime.Bootstrap.dll")
Copy-Item -LiteralPath $DirectMlPath -Destination (Join-Path $StageDir "DirectML.dll")

if (Test-Path -LiteralPath $ResourcesPri) {
    Copy-Item -LiteralPath $ResourcesPri -Destination (Join-Path $StageDir "resources.pri")
}

# README stub inside package
$packReadme = @"
# Translator Overlay $Version (portable)

## Requirements
- Windows 10/11 x64
- Windows App Runtime (framework-dependent)
  https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads
- Microsoft Visual C++ Redistributable
- Network for first-run OCR model download (oar-ocr) and translation API

## Run
1. Extract this folder anywhere.
2. Double-click translator-app.exe
3. Configure API key in the UI and Save.

config.toml and models/ are created next to the executable on first run.
"@
Set-Content -LiteralPath (Join-Path $StageDir "README.txt") -Value $packReadme -Encoding utf8

Write-Host "==> Staging contents:"
Get-ChildItem -LiteralPath $StageDir -Recurse -File |
    ForEach-Object {
        $rel = $_.FullName.Substring($StageDir.Length).TrimStart('\')
        $mb = [math]::Round($_.Length / 1MB, 2)
        "  $rel  ($mb MB)"
    }

# Fail if required sidecars are missing from the stage (guard against future script drift).
$requiredSidecars = @(
    "translator-app.exe",
    "Microsoft.WindowsAppRuntime.Bootstrap.dll",
    "DirectML.dll"
)
foreach ($name in $requiredSidecars) {
    $p = Join-Path $StageDir $name
    if (-not (Test-Path -LiteralPath $p)) {
        throw "Staging incomplete: missing $name"
    }
}

if (Test-Path -LiteralPath $ZipPath) {
    Remove-Item -LiteralPath $ZipPath -Force
}

Write-Host "==> Creating ZIP..."
# Compress stage folder so extract yields TranslatorOverlay\
Compress-Archive -Path $StageDir -DestinationPath $ZipPath -CompressionLevel Optimal

$zipMb = [math]::Round((Get-Item -LiteralPath $ZipPath).Length / 1MB, 2)
Write-Host ""
Write-Host "Done."
Write-Host "  Stage: $StageDir"
Write-Host "  Zip:   $ZipPath  ($zipMb MB)"
Write-Host ""
Write-Host "Note: Users need Windows App Runtime installed before launching."
