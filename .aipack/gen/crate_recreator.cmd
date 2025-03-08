@echo off
REM Crate Recreator Batch Wrapper
REM This batch file calls the corresponding Python script with the same base name.
REM For example, if this file is named "crate_recreator.cmd", it will run "crate_recreator.py".
REM All arguments passed to this .cmd file are forwarded to the Python script.

REM Get the name of this script without extension.
set "SCRIPT_NAME=%~n0"

REM Build the path to the Python script (assumed to be in the same directory).
set "PY_SCRIPT=%~dp0%SCRIPT_NAME%.py"

REM Check if the Python script exists.
if not exist "%PY_SCRIPT%" (
    echo [ERROR] Python script "%PY_SCRIPT%" not found.
    pause
    exit /b 1
)

REM Execute the Python script with all passed arguments.
python "%PY_SCRIPT%" %*

