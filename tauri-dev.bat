@echo off
setlocal

REM Auto-detect MSVC Build Tools via vswhere
set "MSVC="
for /f "usebackq tokens=*" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x64.x86 -property installationPath 2^>nul`) do (
    for /f "usebackq tokens=*" %%j in (`dir /b /ad "%%i\VC\Tools\MSVC\" 2^>nul`) do (
        set "MSVC=%%i\VC\Tools\MSVC\%%j"
    )
)

REM Fallback: check common paths
if not defined MSVC (
    for %%d in (C D E) do (
        for %%p in (
            "%%d:\Program Files\MSBuildTools"
            "%%d:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
            "%%d:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools"
            "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
        ) do (
            if exist "%%~p\VC\Tools\MSVC" (
                for /f "usebackq tokens=*" %%v in (`dir /b /ad "%%~p\VC\Tools\MSVC\"`) do (
                    set "MSVC=%%~p\VC\Tools\MSVC\%%v"
                    goto :msvc_found
                )
            )
        )
    )
)
:msvc_found

if not defined MSVC (
    echo ERROR: MSVC Build Tools not found.
    echo Please install Visual Studio Build Tools with C++ workload.
    exit /b 1
)

REM Auto-detect Windows SDK
set "SDK=C:\Program Files (x86)\Windows Kits\10"
set "SDK_VER=10.0.26100.0"
if not exist "%SDK%\Include\%SDK_VER%" (
    for /f "usebackq tokens=*" %%v in (`dir /b /ad "%SDK%\Include\" 2^>nul ^| sort /r`) do (
        set "SDK_VER=%%v"
        goto :found_sdk
    )
    echo ERROR: Windows SDK not found.
    exit /b 1
)
:found_sdk

REM Set environment
set PATH=%MSVC%\bin\HostX64\x64;%SDK%\bin\%SDK_VER%\x64;%PATH%
set INCLUDE=%MSVC%\include;%SDK%\Include\%SDK_VER%\ucrt;%SDK%\Include\%SDK_VER%\um;%SDK%\Include\%SDK_VER%\shared
set LIB=%MSVC%\lib\x64;%SDK%\Lib\%SDK_VER%\um\x64;%SDK%\Lib\%SDK_VER%\ucrt\x64

cd /d "%~dp0"
npx tauri dev
