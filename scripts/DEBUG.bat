@echo off
setlocal enabledelayedexpansion
cls
color 0E
title DEBUG - GhostHandDesk

set "ROOT_DIR=%~dp0"

echo ========================================
echo   MODE DEBUG - AFFICHAGE COMPLET
echo ========================================
echo.

REM Nettoyer
echo Nettoyage des processus...
taskkill /F /IM signaling-server.exe >nul 2>&1
taskkill /F /IM ghosthanddesk-tauri.exe >nul 2>&1
timeout /t 2 /nobreak >nul
echo.

REM Trouver un port libre
echo Recherche d'un port libre...
set PORT_FOUND=0
for /L %%P in (9000,1,9050) do (
    netstat -an | findstr ":%%P " >nul
    if errorlevel 1 (
        set "SERVER_PORT=%%P"
        set PORT_FOUND=1
        echo [OK] Port libre trouve: %%P
        goto :port_found
    )
)

:port_found
if !PORT_FOUND!==0 (
    echo [ERREUR] Aucun port libre !
    pause
    exit /b 1
)

echo.
echo Port utilise: !SERVER_PORT!
echo !SERVER_PORT! > "%ROOT_DIR%server_port.txt"
echo.

REM Lancer le serveur EN PREMIER PLAN pour voir les erreurs
echo ========================================
echo LANCEMENT DU SERVEUR
echo ========================================
echo Port: !SERVER_PORT!
echo Appuyez sur CTRL+C pour arreter
echo.
cd /d "%ROOT_DIR%server"
set SERVER_HOST=:!SERVER_PORT!
signaling-server.exe

pause
