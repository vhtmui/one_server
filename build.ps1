cargo build --release

$releaseDir = "release"
if (Test-Path $releaseDir) { Remove-Item $releaseDir -Recurse -Force }
New-Item -ItemType Directory -Path "$releaseDir\asset" | Out-Null

$exeName = "one_server.exe"
Copy-Item ".\target\release\$exeName" "$releaseDir\$exeName"
Copy-Item ".\asset\cfg.json" "$releaseDir\asset"
Copy-Item ".\asset\launch.bat" "$releaseDir"

Write-Host "Release 构建并打包完成，输出目录: $releaseDir"