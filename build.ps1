# build.ps1 - 设置 MSVC 环境并运行 cargo
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
$projectDir = "G:\Projnew\ai\keymapper\src-tauri"

# 调用 vcvarsall.bat 设置环境变量
cmd /c "`"$vcvars`" x64 && set" | ForEach-Object {
    if ($_ -match "^(.+?)=(.*)$") {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], "Process")
    }
}

cd $projectDir

if ($args[0] -eq "build") {
    cargo build --release
} elseif ($args[0] -eq "dev") {
    cargo build
} else {
    cargo check
}
