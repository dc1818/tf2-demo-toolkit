@echo off
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust/Cargo was not found. Install it from https://rustup.rs/ and reopen this terminal.
  pause
  exit /b 1
)

cargo build --workspace --release
if errorlevel 1 (
  echo.
  echo BUILD FAILED. The first compiler error above is the useful one.
  pause
  exit /b 1
)

if not exist dist mkdir dist
copy /y target\release\tf2-demo-toolkit.exe dist\TF2_Demo_Toolkit.exe >nul
copy /y target\release\tf2-demo-director.exe dist\TF2_Demo_Director.exe >nul
copy /y target\release\export_all.exe dist\export_all.exe >nul
if exist dist\recording_resources_archive rmdir /s /q dist\recording_resources_archive
xcopy /e /i /y recording_resources_archive dist\recording_resources_archive >nul
echo.
echo Built TF2 Demo Toolkit, TF2 Demo Director, the parser, and recording resources in dist\
echo Open dist\TF2_Demo_Toolkit.exe to start the program.
pause
