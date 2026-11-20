$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$vendorPath = Join-Path $projectRoot 'vendor'
New-Item -ItemType Directory -Path $vendorPath -Force | Out-Null
$archivePath = Join-Path $vendorPath 'stockfish.zip'
Invoke-WebRequest 'https://github.com/official-stockfish/Stockfish/releases/download/sf_19/stockfish-windows-x86-64-universal.zip' -OutFile $archivePath
Expand-Archive -LiteralPath $archivePath -DestinationPath $vendorPath -Force
Remove-Item -LiteralPath $archivePath
Write-Output 'Stockfish 19 installed, including upstream source and license.'
