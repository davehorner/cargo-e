@echo off
setlocal

set NO_COLOR=1
echo HORNERs qc_cap.cmd is running...

REM Save the original working directory (where Cargo.toml is expected)
set "ORIG_DIR=%CD%"

REM --- Check for Cargo.toml in the original working directory ---
if not exist "%ORIG_DIR%\Cargo.toml" (
    echo Cargo.toml not found in %ORIG_DIR%.
    pause
    exit /b 1
)

REM --- Get the Cargo Package Name via cargo-get ---
for /F "usebackq delims=" %%N in (`cargo get package.name --entry="%ORIG_DIR%"`) do set "pkgname=%%N"

REM --- Get the Pretty Version via cargo-get ---
for /F "usebackq delims=" %%V in (`cargo get package.version --pretty --entry="%ORIG_DIR%"`) do set "pkgversion=%%V"

REM Replace spaces with underscores in the extracted values (if any)
set "pkgname=%pkgname: =_%"
set "pkgversion=%pkgversion: =_%"

REM --- Generate a timestamp using PowerShell (format: YYMMDD_HHMMSS) ---
for /F "delims=" %%a in ('powershell -NoProfile -Command "Get-Date -Format \"yyMMdd_HHmmss\""' ) do set "datetime=%%a"

REM Build the full logfile path in the original working directory.
set "logfile=%ORIG_DIR%\qc_%pkgname%_%pkgversion%_%datetime%.log"

REM Set flags so qc.cmd won’t pause for interactive input and will skip launching bacon/cbacon.
set SKIP_PAUSE=1
set IS_CAPTURE=1

echo Running qc.cmd and logging to %logfile%...
REM Call qc.cmd using its full path (located in the same folder as qc_cap.cmd)
call "%~dp0qc.cmd" 1>"%logfile%" 2>&1

echo Finished running qc.cmd.

REM --- Check if we should display the log ---
if defined QC_NO_BAT (
    echo QC_NO_BAT is defined. Skipping log display.
) else (
    choice /M "Do you want to display the log?" /C YN >nul
    if errorlevel 2 (
        echo Not displaying the log.
    ) else (
        echo Displaying log:
        type "%logfile%"
    )
)

pause
endlocal

