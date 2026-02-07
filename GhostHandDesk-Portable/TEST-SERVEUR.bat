@echo off
:: Script de test du serveur seul

cd /d "%~dp0"

echo.
echo ========================================
echo   Test Serveur GhostHandDesk
echo ========================================
echo.

:: Tuer processus existants
taskkill /F /IM signaling-server.exe >nul 2>&1

echo Configuration :
echo   - REQUIRE_TLS=false
echo   - PORT=9000
echo   - DISABLE_ORIGIN_CHECK=true
echo.

:: Configurer et lancer
set REQUIRE_TLS=false
set PORT=9000
set DISABLE_ORIGIN_CHECK=true

echo Lancement du serveur...
signaling-server.exe

:: Le serveur s'exécute ici jusqu'à Ctrl+C
