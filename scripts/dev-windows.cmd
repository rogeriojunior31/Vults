@echo off
setlocal
cd /d "%~dp0.."

rem Keep every environment change local to this launcher.
for /d %%N in ("target\local-tools\node-*") do if exist "%%N\node.exe" set "PATH=%%~fN;%PATH%"
set "VULTS_VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VULTS_VSWHERE%" goto missing_visual_studio
for /f "usebackq tokens=*" %%V in (`"%VULTS_VSWHERE%" -latest -prerelease -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VULTS_VS=%%V"
if not defined VULTS_VS goto missing_visual_studio
call "%VULTS_VS%\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b 1
set "PATH=%VULTS_VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%VULTS_VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%PATH%"
set "CMAKE_GENERATOR=Ninja"
set "CMAKE_GENERATOR_INSTANCE="

if not defined LIBCLANG_PATH if exist "%ProgramFiles%\LLVM\bin\libclang.dll" set "LIBCLANG_PATH=%ProgramFiles%\LLVM\bin"
if not defined LIBCLANG_PATH if exist "%VULTS_VS%\VC\Tools\Llvm\x64\bin\libclang.dll" set "LIBCLANG_PATH=%VULTS_VS%\VC\Tools\Llvm\x64\bin"
for /d %%L in ("target\local-tools\libclang\*.data") do if not defined LIBCLANG_PATH if exist "%%L\platlib\clang\native\libclang.dll" set "LIBCLANG_PATH=%%~fL\platlib\clang\native"

set "VULTS_MISSING="
where node.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% Node.js"
where npm.cmd >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% npm"
where cargo.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% Rust"
where cl.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% MSVC"
where link.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% MSVC-linker"
where rc.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% Windows-SDK"
where cmake.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% CMake"
where ninja.exe >nul 2>nul || set "VULTS_MISSING=%VULTS_MISSING% Ninja"
if not exist "%LIBCLANG_PATH%\libclang.dll" set "VULTS_MISSING=%VULTS_MISSING% libclang"
set "VULTS_WEBVIEW="
reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv >nul 2>nul && set "VULTS_WEBVIEW=1"
reg query "HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv >nul 2>nul && set "VULTS_WEBVIEW=1"
reg query "HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv >nul 2>nul && set "VULTS_WEBVIEW=1"
if not defined VULTS_WEBVIEW set "VULTS_MISSING=%VULTS_MISSING% WebView2-Runtime"
if defined VULTS_MISSING goto missing_tools

if "%~1"=="--check" exit /b 0
call npm.cmd run tauri dev
exit /b %errorlevel%

:missing_visual_studio
echo Install Visual Studio Desktop development with C++, including MSVC x64/x86, Windows SDK and CMake tools.
exit /b 1

:missing_tools
echo Missing prerequisites:%VULTS_MISSING%
echo Install LLVM for libclang.dll, or set LIBCLANG_PATH to its directory.
exit /b 1
