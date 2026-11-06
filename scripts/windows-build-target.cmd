@echo off
rem Only the caller that creates a mapping may remove it, after its child exits.
set "VULTS_BUILD_DRIVE="
if defined CARGO_TARGET_DIR exit /b 0
for %%D in (V W X Y Z) do if not defined CARGO_TARGET_DIR call :reserve %%D
if defined CARGO_TARGET_DIR exit /b 0
echo No free build drive V-Z. Set CARGO_TARGET_DIR to a short path and retry.
exit /b 1
:reserve
set "VULTS_BUILD_CACHE=%CD%\target\gpu-build-%1"
if /i "%1"=="V" set "VULTS_BUILD_CACHE=%CD%\target\gpu-build"
if exist %1:\ (
  for /f "tokens=1,2,*" %%A in ('subst') do if /i "%%A"=="%1:\:" if /i "%%C"=="%VULTS_BUILD_CACHE%" set "CARGO_TARGET_DIR=%1:\"
  exit /b 0
)
if not exist "%VULTS_BUILD_CACHE%" mkdir "%VULTS_BUILD_CACHE%"
subst %1: "%VULTS_BUILD_CACHE%" >nul 2>nul
if errorlevel 1 exit /b 0
set "VULTS_BUILD_DRIVE=%1:"
set "CARGO_TARGET_DIR=%1:\"
exit /b 0
