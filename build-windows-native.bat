@echo off
REM Build script for Windows (native compilation on Windows)
REM This script builds the wz-viewer application for Windows

echo Building wz-viewer for Windows...

REM Install Windows target if not already installed
echo Installing Windows target...
rustup target add x86_64-pc-windows-msvc

REM Build for Windows
echo Compiling for Windows (x86_64-pc-windows-msvc)...
cargo build --target x86_64-pc-windows-msvc --release --bin wz-viewer

REM Create output directory
if not exist "dist\windows" mkdir "dist\windows"
set OUTPUT_DIR=dist\windows

REM Copy the executable
echo Copying executable...
copy "target\x86_64-pc-windows-msvc\release\wz-viewer.exe" "%OUTPUT_DIR%\"

echo Build complete!
echo Windows executable: %OUTPUT_DIR%\wz-viewer.exe
