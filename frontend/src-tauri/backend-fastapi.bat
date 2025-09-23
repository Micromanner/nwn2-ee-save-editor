@echo off
REM FastAPI Backend Sidecar for NWN2 Save Editor
REM This script is launched by Tauri as an external binary

set BACKEND_DIR=%~dp0..\..\backend
set PYTHON_EXE=%BACKEND_DIR%\venv\Scripts\python.exe
set FASTAPI_SCRIPT=%BACKEND_DIR%\fastapi_server.py

REM Environment variables are loaded from backend/.env file via python-dotenv

REM Change to backend directory
cd /d "%BACKEND_DIR%"

REM Check if Python executable exists
if not exist "%PYTHON_EXE%" (
    echo Error: Python virtual environment not found at %PYTHON_EXE%
    exit /b 1
)

REM Check if FastAPI script exists
if not exist "%FASTAPI_SCRIPT%" (
    echo Error: FastAPI script not found at %FASTAPI_SCRIPT%
    exit /b 1
)

REM Launch FastAPI backend
echo Starting NWN2 Save Editor FastAPI backend on %HOST%:%PORT%
"%PYTHON_EXE%" "%FASTAPI_SCRIPT%"