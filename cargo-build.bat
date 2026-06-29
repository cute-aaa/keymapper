@echo off
setlocal

set MSVC=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207
set SDK=C:\Program Files (x86)\Windows Kits\10
set SDK_VER=10.0.26100.0
set SDL2_DIR=G:\Projnew\ai\keymapper\SDL2

set PATH=%MSVC%\bin\HostX64\x64;%SDK%\bin\%SDK_VER%\x64;%PATH%
set INCLUDE=%MSVC%\include;%SDK%\Include\%SDK_VER%\ucrt;%SDK%\Include\%SDK_VER%\um;%SDK%\Include\%SDK_VER%\shared;%SDL2_DIR%\include
set LIB=%MSVC%\lib\x64;%SDK%\Lib\%SDK_VER%\um\x64;%SDK%\Lib\%SDK_VER%\ucrt\x64;%SDL2_DIR%\lib\x64

cd /d G:\Projnew\ai\keymapper\src-tauri
echo Starting cargo build...
cargo build 2>&1
echo.
echo DONE EXIT_CODE=%ERRORLEVEL%
