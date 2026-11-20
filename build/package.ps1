param([string]$Binary = 'target/release/chess-assistant-pro.exe')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$packagePath = Join-Path $projectRoot 'release/RbChessAssist'
New-Item -ItemType Directory -Path $packagePath -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot $Binary) -Destination (Join-Path $packagePath 'RbChessAssist.exe')
Copy-Item -LiteralPath (Join-Path $projectRoot 'vendor/stockfish') -Destination $packagePath -Recurse -Force
foreach ($file in @('README.md','LICENSE','THIRD_PARTY.md')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $file) -Destination $packagePath
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs') -Destination $packagePath -Recurse -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'assets/books') -Destination $packagePath -Recurse -Force
# Include the license files of dependencies fetched by Cargo.
$licensePath = Join-Path $packagePath 'third-party-licenses'
New-Item -ItemType Directory -Path $licensePath -Force | Out-Null
$registryPath = Join-Path $env:USERPROFILE '.cargo/registry/src'
if (Test-Path -LiteralPath $registryPath) {
    Get-ChildItem -LiteralPath $registryPath -Directory | ForEach-Object {
        Get-ChildItem -LiteralPath $_.FullName -Directory | ForEach-Object {
            $cratePath = $_
            Get-ChildItem -LiteralPath $cratePath.FullName -File | Where-Object Name -Match '^(LICENSE|COPYING|NOTICE|COPYRIGHT)' | ForEach-Object {
                $destination = Join-Path $licensePath $cratePath.Name
                New-Item -ItemType Directory -Path $destination -Force | Out-Null
                Copy-Item -LiteralPath $_.FullName -Destination $destination
            }
        }
    }
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'assets/fonts/OFL.txt') -Destination (Join-Path $licensePath 'Inter-OFL.txt')
Compress-Archive -Path "$packagePath/*" -DestinationPath (Join-Path $projectRoot 'release/RbChessAssist-Windows.zip') -Force
Write-Output "Packaged application at $packagePath"
