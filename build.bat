@echo off
rem PRETTY_RUN_BEGIN
set "NOPAUSE=1"
if not defined PRETTY_MODE set "PRETTY_MODE=build"
set "PRETTY_PY=python"
where py >nul 2>&1 && set "PRETTY_PY=py -3"
if not defined PRETTY_RUN if exist "%~dp0tools\pretty_run.py" (
    set "PRETTY_RUN=1"
    %PRETTY_PY% "%~dp0tools\pretty_run.py" --wrap --mode %PRETTY_MODE% -- cmd /d /c "%~f0" %*
    exit /b
)
rem PRETTY_RUN_END
rem Builds EH3D-PHOENIX-LASER (release).
cd /d "%~dp0"
set "TARGET_DIR=target"
if defined CARGO_TARGET_DIR set "TARGET_DIR=%CARGO_TARGET_DIR%"
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust is not installed. Get it from https://rustup.rs
    goto :end
)
cargo build --release
if errorlevel 1 (
    echo.
    echo Build FAILED.
) else (
    echo.
    echo Build OK: %TARGET_DIR%\release\phoenix-laser.exe
)
:end
if not defined NOPAUSE pause
