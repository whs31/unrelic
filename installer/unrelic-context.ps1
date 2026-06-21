param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string] $InputPath
)

$ErrorActionPreference = "Continue"

$scriptDir = Split-Path -Parent $PSCommandPath
$unrelic = Join-Path $scriptDir "unrelic.exe"

if (-not (Test-Path -LiteralPath $unrelic)) {
  Write-Error "Could not find unrelic.exe next to this script: $unrelic"
  Read-Host "Press Enter to close"
  exit 1
}

if (-not (Test-Path -LiteralPath $InputPath)) {
  Write-Error "Selected path does not exist: $InputPath"
  Read-Host "Press Enter to close"
  exit 1
}

& $unrelic $InputPath
$exitCode = $LASTEXITCODE

if ($exitCode -ne 0) {
  Write-Host ""
  Write-Host "unrelic failed with exit code $exitCode."
}

Write-Host ""
Read-Host "Press Enter to close"
exit $exitCode
