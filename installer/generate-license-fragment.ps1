param(
  [Parameter(Mandatory = $true)]
  [string] $PayloadDir,

  [Parameter(Mandatory = $true)]
  [string] $OutputPath
)

$ErrorActionPreference = "Stop"

function ConvertTo-WixId {
  param([string] $Value)

  $sanitized = $Value -replace "[^A-Za-z0-9_\.]", "_"
  if ($sanitized -notmatch "^[A-Za-z_]") {
    $sanitized = "_$sanitized"
  }
  return $sanitized
}

function New-StableGuid {
  param([string] $Value)

  $md5 = [System.Security.Cryptography.MD5]::Create()
  $bytes = $md5.ComputeHash([System.Text.Encoding]::UTF8.GetBytes("unrelic-ffmpeg-license:$Value"))
  $bytes[6] = ($bytes[6] -band 0x0f) -bor 0x30
  $bytes[8] = ($bytes[8] -band 0x3f) -bor 0x80
  return ([Guid]::new($bytes)).ToString().ToUpperInvariant()
}

function Escape-Xml {
  param([string] $Value)

  return [System.Security.SecurityElement]::Escape($Value)
}

$licenseDir = Join-Path $PayloadDir "ffmpeg-licenses"
if (-not (Test-Path $licenseDir)) {
  throw "FFmpeg license directory does not exist: $licenseDir"
}

$files = @(Get-ChildItem $licenseDir -File | Sort-Object Name)
if ($files.Count -eq 0) {
  throw "No FFmpeg license files were staged in: $licenseDir"
}

$componentLines = New-Object System.Collections.Generic.List[string]
$componentRefLines = New-Object System.Collections.Generic.List[string]
$index = 0

foreach ($file in $files) {
  $relativeName = $file.Name
  $idSuffix = ConvertTo-WixId ("{0:D3}_{1}" -f $index, $relativeName)
  $componentId = "FfmpegLicenseComponent_$idSuffix"
  $fileId = "FfmpegLicenseFile_$idSuffix"
  $keyPathName = "FfmpegLicenseComponent_$idSuffix"
  $guid = New-StableGuid $relativeName
  $source = Escape-Xml $file.FullName
  $relativeValue = Escape-Xml $relativeName

  $componentLines.Add("      <Component Id=`"$componentId`" Guid=`"{$guid}`">")
  $componentLines.Add("        <File Id=`"$fileId`" Source=`"$source`" />")
  $componentLines.Add("        <RegistryValue Root=`"HKCU`" Key=`"Software\unrelic\Installer\FfmpegLicenses`" Name=`"$keyPathName`" Value=`"$relativeValue`" Type=`"string`" KeyPath=`"yes`" />")
  $componentLines.Add("      </Component>")
  $componentRefLines.Add("      <ComponentRef Id=`"$componentId`" />")
  $index += 1
}

$content = @"
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Fragment>
    <DirectoryRef Id="FfmpegLicensesFolder">
$($componentLines -join "`n")
    </DirectoryRef>
  </Fragment>

  <Fragment>
    <ComponentGroup Id="FfmpegLicenseComponents">
$($componentRefLines -join "`n")
    </ComponentGroup>
  </Fragment>
</Wix>
"@

Set-Content -Path $OutputPath -Value $content -Encoding UTF8
