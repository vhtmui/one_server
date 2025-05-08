# 先创建父目录
New-Item -ItemType Directory -Path ".\  special path" -Force

# 创建子目录
New-Item -ItemType Directory -Path ".\  special path\Mix!@#$%^&()=+{}[];',~`_目录" -Force

# 创建孙目录
New-Item -ItemType Directory -Path ".\  special path\Mix!@#$%^&()=+{}[];',~`_目录\Sub Folder 中间 空 格" -Force

# 创建文件
New-Item -ItemType File -Path ".\  special path\Mix!@#$%^&()=+{}[];',~`_目录\Sub Folder 中间 空 格\  文件_🌟Unicode_引号_&_Sp  ecial_Chars_最终版_v2.0%20@2024  " -Force