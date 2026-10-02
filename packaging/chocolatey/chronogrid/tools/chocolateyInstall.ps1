$ErrorActionPreference = 'Stop'

$packageName = 'chronogrid'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$version = '0.2.0'
$releaseBase = "https://github.com/karencohenjw/chronogrid/releases/download/v$version"
$url32 = "$releaseBase/chronogrid-$version-windows-i686.zip"
$url64 = "$releaseBase/chronogrid-$version-windows-x86_64.zip"
$checksum32 = 'd66ba1fdf26da61750b60c5cbeb150aed1ea932dde482ba6cfee12dc7cc9265a'
$checksum64 = '0ff0c1e736c31b8ccd1942354ce85f107281f95903e8216f207eed2cf4cd5662'

Install-ChocolateyZipPackage `
  -PackageName $packageName `
  -Url $url32 `
  -Checksum $checksum32 `
  -ChecksumType 'sha256' `
  -Url64bit $url64 `
  -Checksum64 $checksum64 `
  -ChecksumType64 'sha256' `
  -UnzipLocation $toolsDir
