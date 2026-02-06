@echo off
cls
color 0A
title GhostHandDesk - Lanceur
echo.
echo ========================================
echo    GHOSTHANDDESK - LANCEMENT COMPLET
echo ========================================
echo.

set "ROOT_DIR=%~dp0"

REM Tuer tous les processus existants
echo [0/4] Nettoyage des processus existants...
taskkill /F /IM signaling-server.exe >nul 2>&1
taskkill /F /IM ghosthanddesk-tauri.exe >nul 2>&1
timeout /t 2 /nobreak >nul
cls

echo.
echo ========================================
echo    GHOSTHANDDESK - LANCEMENT COMPLET
echo ========================================
echo.

REM Lancer le serveur sur le port fixe 9080
echo [1/4] Demarrage du serveur sur PORT FIXE 9080...
start "SERVEUR GhostHandDesk - NE PAS FERMER" /MIN cmd /k "cd /d %ROOT_DIR%server && set SERVER_HOST=:9080 && signaling-server.exe"
timeout /t 5 /nobreak >nul

REM Verifier que le serveur tourne
netstat -an | findstr ":9080" >nul
if errorlevel 1 (
    color 0C
    echo.
    echo [ERREUR] Le serveur n'a pas demarre sur le port 9080 !
    echo Verifiez que le port n'est pas deja utilise.
    echo.
    pause
    exit /b 1
)
echo [OK] Serveur actif sur ws://localhost:9080/ws
echo.

REM Lancer la premiere instance
echo [2/4] Lancement Instance 1 (PARTAGE son ecran)...
echo Toutes les instances utilisent le MEME serveur (port 9080)
start "GhostHandDesk - Instance 1 (PARTAGE)" /D "%ROOT_DIR%client\src-tauri\target\release" ghosthanddesk-tauri.exe
timeout /t 4 /nobreak >nul

REM Lancer la deuxieme instance
echo [3/4] Lancement Instance 2 (PREND le controle)...
start "GhostHandDesk - Instance 2 (CONTROLE)" /D "%ROOT_DIR%client\src-tauri\target\release" ghosthanddesk-tauri.exe
timeout /t 2 /nobreak >nul

echo.
echo [4/4] Lancement termine !
color 0B
echo.
echo ========================================
echo       INSTRUCTIONS DE TEST
echo ========================================
echo.
echo Vous devez avoir 3 fenetres ouvertes:
echo   [1] Serveur (minimisee - ne pas fermer !)
echo   [2] Instance 1 (PARTAGE)
echo   [3] Instance 2 (CONTROLE)
echo.
echo === ETAPE 1 ===
echo Dans "Instance 1 - PARTAGE"
echo   ^> Copiez le Device ID (bouton copier)
echo.
echo === ETAPE 2 ===
echo Dans "Instance 2 - CONTROLE"
echo   ^> Cliquez "Se connecter a un appareil distant"
echo   ^> Collez le Device ID de l'Instance 1
echo   ^> Cliquez "Se connecter"
echo.
echo === ETAPE 3 ===
echo Dans "Instance 1 - PARTAGE"
echo   ^> Un popup apparait "Demande de Connexion"
echo   ^> Cliquez "Accepter"
echo.
echo === ETAPE 4 - RESULTAT ===
echo   ^> Instance 2 affiche l'ecran de Instance 1
echo   ^> Vous pouvez controler a distance !
echo.
echo ========================================
echo.
echo Pour arreter:
echo   1. Fermez les 2 instances
echo   2. Fermez la fenetre SERVEUR
echo.
pause
