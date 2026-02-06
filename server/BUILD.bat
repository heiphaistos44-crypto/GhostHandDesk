@echo off
title Compilation Serveur Go
color 0B

echo ========================================
echo   COMPILATION SERVEUR GHOSTHANDDESK
echo ========================================
echo.

cd /d "%~dp0"

echo Compilation en cours...
go build -o signaling-server.exe ./cmd/signaling

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ========================================
    echo   COMPILATION REUSSIE !
    echo ========================================
    echo.
    echo Le serveur a ete compile : signaling-server.exe
    echo.
) else (
    echo.
    echo ========================================
    echo   ERREUR DE COMPILATION
    echo ========================================
    echo.
    pause
    exit /b 1
)

pause
