$ErrorActionPreference = 'Stop'

$packageName = 'chronogrid'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$version = '0.2.0'
$releaseBase = "https://github.com/karencohenjw/chronogrid/releases/download/v$version"
$url32 = "$releaseBase/chronogrid-$version-windows-i686.zip"
$url64 = "$releaseBase/chronogrid-$version-windows-x86_64.zip"
$checksum32 = 'eef5c2fa71c84a04468b2ef83abc9d77ea3ef25dcd976bb9750c5e92dca5df6c'
$checksum64 = '412cc33c58e7f8b017167808011bf6f7bd024d9c383bfe479c8df25bc962d3de'

Install-ChocolateyZipPackage `
  -PackageName $packageName `
  -Url $url32 `
  -Checksum $checksum32 `
  -ChecksumType 'sha256' `
  -Url64bit $url64 `
  -Checksum64 $checksum64 `
  -ChecksumType64 'sha256' `
  -UnzipLocation $toolsDir
