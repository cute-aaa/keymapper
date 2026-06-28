$ErrorActionPreference = "Continue"

# Source vcvars
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
$output = cmd /c "`"$vcvars`" x64 2>&1 && set"
foreach ($line in $output) {
    if ($line -match "^([^=]+)=(.+)$") {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
    }
}

cd "G:\Projnew\ai\keymapper\src-tauri"
Write-Output "Starting cargo check..."
cargo check 2>&1
Write-Output "DONE EXIT_CODE=$LASTEXITCODE"
