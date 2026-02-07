@echo off
:: Script de test avec logs DEBUG détaillés

cd /d "%~dp0"

echo.
echo ========================================
echo   TEST DEBUG - GhostHandDesk
echo ========================================
echo.

:: Tuer processus existants
echo Nettoyage processus existants...
taskkill /F /IM signaling-server.exe >nul 2>&1
taskkill /F /IM ghosthanddesk-tauri.exe >nul 2>&1
timeout /t 1 >nul

echo.
echo Configuration serveur :
echo   REQUIRE_TLS=false
echo   PORT=9000
echo   DISABLE_ORIGIN_CHECK=true
echo   LOG_LEVEL=debug
echo.

:: Configurer
set REQUIRE_TLS=false
set PORT=9000
set DISABLE_ORIGIN_CHECK=true
set LOG_LEVEL=debug

echo ========================================
echo SERVEUR DÉMARRÉ - Surveillez les logs
echo ========================================
echo.
echo LOGS À SURVEILLER :
echo   [HUB] Client enregistré: GHD-xxx
echo   [CLIENT GHD-xxx] Message reçu: ConnectRequest
echo   [HUB] Demande de connexion transférée de X vers Y
echo.
echo Ctrl+C pour arrêter
echo ========================================
echo.

signaling-server.exe
