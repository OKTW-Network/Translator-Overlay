param([switch]$Force)

$ErrorActionPreference = "Stop"

$OrtTag = "v1.30.0"
$RequiredNames = @(
    "onnxruntime.dll",
    "onnxruntime.lib",
    "DirectML.dll",
    "webgpu_dawn.dll",
    "dxcompiler.dll",
    "dxil.dll"
)

function Test-Staged {
    param([string]$Dir)
    if (-not (Test-Path -LiteralPath $Dir)) {
        return $false
    }
    foreach ($name in $RequiredNames) {
        if (-not (Test-Path -LiteralPath (Join-Path $Dir $name))) {
            return $false
        }
    }
    $got = Get-Content -LiteralPath (Join-Path $Dir "VERSION") -ErrorAction SilentlyContinue
    return ($got -eq $OrtTag)
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$OutDir = Join-Path $Root "third_party\onnxruntime-win-x64"
$SourceDir = Join-Path $Root "third_party\onnxruntime-src"
# In ORT 1.30, Dawn checks DXC out at third_party/directx-shader-compiler/src,
# where 1.28 used third_party/dxc. On GitHub Actions the longer path under
# D:\a\Translator-Overlay\Translator-Overlay\... overflows MAX_PATH, so
# utils/llvm-build/llvm-build goes missing and CMake reports ENOENT.
# GitHub Actions sets RUNNER_TEMP to D:\a\_temp. Local builds keep the in-tree build dir.
if ($env:RUNNER_TEMP) {
    $BuildDir = Join-Path $env:RUNNER_TEMP "ort-build"
} else {
    $BuildDir = Join-Path $SourceDir "build"
}

if (-not $Force -and (Test-Staged $OutDir)) {
    Write-Host "Already staged at $OutDir"
    exit 0
}

Write-Host "==> Repo:   $Root"
Write-Host "==> Source: $SourceDir"
Write-Host "==> Build:  $BuildDir"
Write-Host "==> Out:    $OutDir"
Write-Host "==> Tag:    $OrtTag"

if (-not (Test-Path -LiteralPath $SourceDir)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $SourceDir) | Out-Null
    Write-Host "==> Cloning onnxruntime $OrtTag..."
    git clone --branch $OrtTag --depth 1 https://github.com/microsoft/onnxruntime.git $SourceDir
    if ($LASTEXITCODE -ne 0) {
        throw "git clone failed with exit code $LASTEXITCODE"
    }
}

Push-Location -LiteralPath $SourceDir
try {
    git config core.longpaths true
    # Dawn's nested DXC clone is a separate repo, so pass core.longpaths through the environment.
    $env:GIT_CONFIG_COUNT = "1"
    $env:GIT_CONFIG_KEY_0 = "core.longpaths"
    $env:GIT_CONFIG_VALUE_0 = "true"
    $head = (git describe --tags --exact-match HEAD 2>$null)
    $switchTag = $head -ne $OrtTag
    if ($switchTag) {
        Write-Host "==> Checking out $OrtTag..."
        git fetch --depth 1 origin tag $OrtTag
        if ($LASTEXITCODE -ne 0) {
            throw "git fetch $OrtTag failed with exit code $LASTEXITCODE"
        }
        git checkout $OrtTag
        if ($LASTEXITCODE -ne 0) {
            throw "git checkout $OrtTag failed with exit code $LASTEXITCODE"
        }
    }
    Write-Host "==> Updating submodules..."
    git submodule update --init --recursive
    if ($LASTEXITCODE -ne 0) {
        throw "git submodule update failed with exit code $LASTEXITCODE"
    }

    # Dawn's DXC FindD3D12 reads WIN10_SDK_PATH from the registry, but the 64-bit
    # KitsRoot10 (Program Files\Windows Kits\10) often has no um\d3d12.h.
    # Point it at the Program Files (x86) kit that has the header.
    $kits = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10"
    if (Test-Path -LiteralPath (Join-Path $kits "Include")) {
        $sdkVer = Get-ChildItem -LiteralPath (Join-Path $kits "Include") -Directory |
            Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "um\d3d12.h") } |
            Sort-Object Name -Descending |
            Select-Object -First 1 -ExpandProperty Name
        if ($sdkVer) {
            $env:WIN10_SDK_PATH = $kits
            $env:WIN10_SDK_VERSION = $sdkVer
            # Dawn CopyWindowsSDKDLL.cmake reads WINDOWSSDKDIR, not WIN10_SDK_PATH.
            $env:WINDOWSSDKDIR = $kits
            Write-Host "==> WIN10_SDK_PATH=$kits"
            Write-Host "==> WIN10_SDK_VERSION=$sdkVer"
        }
    }

    $stagedVersion = Get-Content -LiteralPath (Join-Path $OutDir "VERSION") -ErrorAction SilentlyContinue
    $versionChanged = $switchTag -or (($null -ne $stagedVersion) -and ($stagedVersion -ne $OrtTag))
    if ($Force -or $versionChanged) {
        # Leftover CMake and Dawn files from another ORT tag or generator break the next configure.
        foreach ($buildRoot in @($BuildDir, (Join-Path $SourceDir "build"))) {
            if (Test-Path -LiteralPath $buildRoot) {
                Write-Host "==> Cleaning $buildRoot"
                Remove-Item -LiteralPath $buildRoot -Recurse -Force
            }
        }
    }

    Write-Host "==> Building ONNX Runtime (DirectML + WebGPU)..."
    # These flags match the Windows WebGPU build in pykeio/ort-artifacts, with ORT 1.30 option names.
    $buildArgs = @(
        "--config", "Release",
        "--parallel",
        "--skip_tests",
        "--use_dml",
        "--use_webgpu",
        "--build_shared_lib",
        "--client_package_build",
        "--compile_no_warning_as_error",
        "--cmake_generator", "Visual Studio 18 2026",
        "--build_dir", $BuildDir,
        "--targets", "onnxruntime",
        "--cmake_extra_defines",
        "onnxruntime_BUILD_DAWN_SHARED_LIBRARY=ON",
        "onnxruntime_ENABLE_DELAY_LOADING_WIN_DLLS=OFF",
        "onnxruntime_BUILD_UNIT_TESTS=OFF"
    )
    if ($env:WIN10_SDK_VERSION) {
        $buildArgs += @("--windows_sdk_version", $env:WIN10_SDK_VERSION)
    }
    & .\build.bat @buildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "onnxruntime build.bat failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$bin = Join-Path $BuildDir "Release\Release"
if (-not (Test-Path -LiteralPath $bin)) {
    throw "No onnxruntime build output at $bin"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Write-Host "==> Staging runtime files..."
foreach ($name in $RequiredNames) {
    $src = Join-Path $bin $name
    if (-not (Test-Path -LiteralPath $src)) {
        throw "Build succeeded but $name was not found in $bin"
    }
    Copy-Item -LiteralPath $src -Destination (Join-Path $OutDir $name) -Force
    Write-Host "  $name  <-  $src"
}
Set-Content -LiteralPath (Join-Path $OutDir "VERSION") -Value $OrtTag -NoNewline

if (-not (Test-Staged $OutDir)) {
    throw "Staging incomplete at $OutDir"
}

Write-Host "Done. Staged at $OutDir"
