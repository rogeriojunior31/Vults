@echo off
rem Called without setlocal so both Windows launchers receive the same compiler environment.
for /d %%N in ("target\local-tools\node-*") do if exist "%%N\node.exe" set "PATH=%%~fN;%PATH%"
set "VULTS_VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VULTS_VSWHERE%" goto missing
for /f "usebackq tokens=*" %%V in (`"%VULTS_VSWHERE%" -latest -prerelease -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VULTS_VS=%%V"
if not defined VULTS_VS goto missing
call "%VULTS_VS%\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b 1
set "PATH=%VULTS_VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%VULTS_VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%PATH%"
set "CMAKE_GENERATOR=Ninja"
set "CMAKE_GENERATOR_INSTANCE="
if not defined LIBCLANG_PATH if exist "%ProgramFiles%\LLVM\bin\libclang.dll" set "LIBCLANG_PATH=%ProgramFiles%\LLVM\bin"
if not defined LIBCLANG_PATH if exist "%VULTS_VS%\VC\Tools\Llvm\x64\bin\libclang.dll" set "LIBCLANG_PATH=%VULTS_VS%\VC\Tools\Llvm\x64\bin"
for /d %%L in ("target\local-tools\libclang\*.data") do if not defined LIBCLANG_PATH if exist "%%L\platlib\clang\native\libclang.dll" set "LIBCLANG_PATH=%%~fL\platlib\clang\native"
exit /b 0
:missing
echo Install Visual Studio Desktop development with C++, including MSVC x64/x86, Windows SDK and CMake tools.
exit /b 1
