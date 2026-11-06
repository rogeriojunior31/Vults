@echo off
setlocal
set "VULTS_BUILD_DRIVE="
cd /d "%~dp0.."

call scripts\windows-build-env.cmd
if errorlevel 1 exit /b 1

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

rem Use an installed or checkout-local SDK without changing the user's environment.
if not defined VULKAN_SDK if exist "target\local-tools\vulkan\Include\vulkan\vulkan.h" set "VULKAN_SDK=%CD%\target\local-tools\vulkan"
if not defined VULKAN_SDK for /d %%V in ("C:\VulkanSDK\*") do if exist "%%V\Bin\glslc.exe" set "VULKAN_SDK=%%~fV"
set "VULTS_VOICE_FEATURES="
if /i not "%~1"=="--cpu" if exist "%VULKAN_SDK%\Bin\glslc.exe" if exist "%VULKAN_SDK%\Include\vulkan\vulkan.h" if exist "%VULKAN_SDK%\Lib\vulkan-1.lib" set "VULTS_VOICE_FEATURES=--features voice-vulkan"
if /i "%~1"=="--gpu" if not defined VULTS_VOICE_FEATURES (
  echo GPU build needs Vulkan SDK with glslc, headers and vulkan-1.lib. Set VULKAN_SDK to its directory.
  exit /b 1
)
if defined VULTS_VOICE_FEATURES (
  set "PATH=%VULKAN_SDK%\Bin;%PATH%"
  echo Voice backend: Vulkan-enabled build. Runtime device availability determines GPU use.
) else (
  echo Voice backend: CPU. Install Vulkan SDK to enable GPU builds, or use --cpu explicitly.
)
if "%~1"=="--check" exit /b 0
if defined VULTS_VOICE_FEATURES call scripts\windows-build-target.cmd
if errorlevel 1 exit /b 1
call npm.cmd run tauri dev -- %VULTS_VOICE_FEATURES%
set "VULTS_EXIT=%errorlevel%"
if defined VULTS_BUILD_DRIVE subst %VULTS_BUILD_DRIVE% /d >nul 2>nul
exit /b %VULTS_EXIT%

:missing_tools
echo Missing prerequisites:%VULTS_MISSING%
echo Install LLVM for libclang.dll, or set LIBCLANG_PATH to its directory.
exit /b 1
