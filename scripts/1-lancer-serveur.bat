@echo off
cls
echo ========================================
echo   Serveur de Signalement GhostHandDesk
echo ========================================
echo.

set "ROOT_DIR=%~dp0"
cd /d "%ROOT_DIR%server"

echo Demarrage du serveur sur le port 9080...
echo.
echo ATTENTION: Gardez cette fenetre ouverte pendant les tests !
echo.
echo Pour lancer l'application:
echo   - Allez dans: %ROOT_DIR%client\src-tauri\target\release\
echo   - Double-cliquez sur ghosthanddesk-tauri.exe
echo   - Vous pouvez le lancer plusieurs fois pour avoir plusieurs instances
echo.
echo ========================================
echo.

signaling-server.exe
