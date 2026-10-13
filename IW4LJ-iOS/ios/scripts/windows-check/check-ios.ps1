# Type-check the iOS build on Windows (no Mac, no Xcode).
#
# This proves the Rust compiles for iOS: every `cfg(target_os = "ios")` path,
# every iOS-only dependency API. It cannot link or run anything, and the C
# parts of dependencies are replaced by empty stand-ins (see fakecc.py).
# A green run here is necessary, not sufficient: the real build is on macOS.
#
#   powershell -ExecutionPolicy Bypass -File check-ios.ps1            # device
#   powershell -ExecutionPolicy Bypass -File check-ios.ps1 -Sim       # simulator
#   powershell -ExecutionPolicy Bypass -File check-ios.ps1 -Package bootstrap
param(
    [switch]$Sim,
    [string]$Package = "iw4l-ios"
)

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = (Resolve-Path (Join-Path $here "..\..\..")).Path
$target = if ($Sim) { "aarch64-apple-ios-sim" } else { "aarch64-apple-ios" }
$key = $target.Replace("-", "_")

$sdk = Join-Path $env:TEMP "iw4lj-fake-ios-sdk"
New-Item -ItemType Directory -Force $sdk | Out-Null

Set-Item "env:CC_$key" (Join-Path $here "fakecc.cmd")
Set-Item "env:CXX_$key" (Join-Path $here "fakecc.cmd")
Set-Item "env:AR_$key" (Join-Path $here "fakear.cmd")
$env:SDKROOT = $sdk
$env:IPHONEOS_DEPLOYMENT_TARGET = "16.0"

Set-Location $repo
cargo check -p $Package --target $target --message-format short
exit $LASTEXITCODE
