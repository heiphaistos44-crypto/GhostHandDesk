@echo off
title SERVEUR GhostHandDesk - NE PAS FERMER
color 0A

REM Créer le fichier de port AVANT de lancer le serveur
echo 9000 > "%~dp0server_port.txt"
echo 9000 > "%~dp0client\src-tauri\target\release\server_port.txt"

cd /d "%~dp0\server"

echo ========================================
echo   SERVEUR DE SIGNALEMENT
echo ========================================
echo.
echo Port: 9000
echo URL: ws://localhost:9000/ws
echo.
echo GARDEZ CETTE FENETRE OUVERTE !
echo.

set SERVER_HOST=:9000
signaling-server.exe

pause
