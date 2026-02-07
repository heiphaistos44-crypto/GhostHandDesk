@echo off
setlocal enabledelayedexpansion
cls
color 0A
title GhostHandDesk - Demarrage Intelligent

set "ROOT_DIR=%~dp0"

echo.
echo ========================================
echo    GHOSTHANDDESK - DEMARRAGE
echo ========================================
echo.

REM Nettoyer les processus existants
echo [1/5] Nettoyage...
taskkill /F /IM signaling-server.exe >nul 2>&1
taskkill /F /IM ghosthanddesk-tauri.exe >nul 2>&1
timeout /t 2 /nobreak >nul

REM Trouver un port libre entre 9000 et 9999
echo [2/5] Recherche d'un port libre...
set PORT_FOUND=0
for /L %%P in (9000,1,9999) do (
    netstat -an | findstr ":%%P " >nul
    if errorlevel 1 (
        set "SERVER_PORT=%%P"
        set PORT_FOUND=1
        goto :port_found
    )
)

:port_found
if !PORT_FOUND!==0 (
    color 0C
    echo ERREUR: Aucun port libre trouve entre 9000-9999 !
    pause
    exit /b 1
)

echo [OK] Port libre trouve: !SERVER_PORT!
echo.

REM Ecrire le port dans un fichier pour les instances
echo !SERVER_PORT! > "%ROOT_DIR%server_port.txt"

REM Lancer le serveur sur le port trouve
echo [3/5] Demarrage du serveur sur le port !SERVER_PORT!...
start "SERVEUR GhostHandDesk" /MIN cmd /k "cd /d %ROOT_DIR%server && set SERVER_HOST=:!SERVER_PORT! && signaling-server.exe"
timeout /t 5 /nobreak >nul

REM Verifier que le serveur est actif
netstat -an | findstr ":!SERVER_PORT! " >nul
if errorlevel 1 (
    color 0C
    echo ERREUR: Le serveur n'a pas demarre !
    pause
    exit /b 1
)

echo [OK] Serveur actif sur ws://localhost:!SERVER_PORT!/ws
echo.

REM Lancer Instance 1
echo [4/5] Lancement Instance 1...
start "GhostHandDesk - Instance 1" cmd /c "set GHD_SERVER_PORT=!SERVER_PORT! && "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe""
timeout /t 4 /nobreak >nul

REM Lancer Instance 2
echo [5/5] Lancement Instance 2...
start "GhostHandDesk - Instance 2" cmd /c "set GHD_SERVER_PORT=!SERVER_PORT! && "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe""
timeout /t 2 /nobreak >nul

color 0B
cls
echo.
echo ========================================
echo        LANCEMENT TERMINE !
echo ========================================
echo.
echo Port du serveur: !SERVER_PORT!
echo URL: ws://localhost:!SERVER_PORT!/ws
echo.
echo Vous avez maintenant:
echo   - 1 Serveur (minimise)
echo   - 2 Instances de GhostHandDesk
echo.
echo ========================================
echo      COMMENT TESTER LA CONNEXION
echo ========================================
echo.
echo 1. Dans Instance 1:
echo    - Copiez le Device ID (bouton copier)
echo.
echo 2. Dans Instance 2:
echo    - Cliquez "Se connecter"
echo    - Collez le Device ID
echo    - Cliquez "Connecter"
echo.
echo 3. Dans Instance 1:
echo    - Acceptez la connexion
echo.
echo 4. Resultat:
echo    - Instance 2 controle Instance 1 !
echo.
echo ========================================
echo.
pause
