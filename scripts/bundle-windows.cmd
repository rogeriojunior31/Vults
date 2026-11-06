@echo off
setlocal
set "VULTS_BUILD_DRIVE="
cd /d "%~dp0.."
call scripts\windows-build-env.cmd
if errorlevel 1 exit /b 1
if not exist "%LIBCLANG_PATH%\libclang.dll" (
  echo Install LLVM or set LIBCLANG_PATH to the directory containing libclang.dll.
  exit /b 1
)
node scripts\prepare-windows-vulkan.mjs
if errorlevel 1 exit /b 1
set "VULKAN_SDK=%CD%\target\windows-vulkan\sdk"
set "PATH=%VULKAN_SDK%\Bin;%PATH%"
call scripts\windows-build-target.cmd
if errorlevel 1 exit /b 1
cargo build --locked --release -p vults-hook
if errorlevel 1 goto failed
node scripts\bundle-windows.mjs %*
set "VULTS_EXIT=%errorlevel%"
goto cleanup
:failed
set "VULTS_EXIT=1"
:cleanup
if defined VULTS_BUILD_DRIVE subst %VULTS_BUILD_DRIVE% /d >nul 2>nul
exit /b %VULTS_EXIT%
