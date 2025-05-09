@echo off
setlocal
REM ran HORNERs qc (fmt, fix, audit)
REM --- Check for Cargo.toml in the current directory ---
if not exist "Cargo.toml" (
    echo Cargo.toml not found in the current directory.
    pause
    exit /b 1
)

echo HORNERs qc.cmd is running...

echo Running cargo fmt...
cargo fmt

echo Running cargo fix --allow-dirty...
cargo fix --allow-dirty

echo Running cargo doc...
cargo doc

echo Running cargo build...
cargo build

echo Running cargo test...
cargo test

echo Running cargo hack check --each-feature --no-dev-deps...
cargo hack check --each-feature --no-dev-deps

echo Running cargo audit...
cargo audit

REM --- Only launch bacon clippy and cbacon if not in capture mode ---
if not defined IS_CAPTURE (
    echo Launching bacon clippy in a new window...
    start cmd /k "bacon clippy && echo Press Enter to exit... && pause"

    echo Launching cbacon in a new window...
    start cmd /k "cbacon && echo Press Enter to exit... && pause"
) else (
    echo IS_CAPTURE is set. Skipping launching bacon clippy and cbacon.
)

echo Horner Quality checks complete.
echo "

REM Only pause if not running in capture mode.
if not defined SKIP_PAUSE (
    pause
)

endlocal

