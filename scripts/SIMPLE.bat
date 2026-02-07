@echo off
cls
color 0A

cd /d "%~dp0"

echo ========================================
echo    LANCEMENT GHOSTHANDDESK
echo ========================================
echo.

REM Tuer tout
taskkill /F /IM signaling-server.exe 2>nul
taskkill /F /IM ghosthanddesk-tauri.exe 2>nul
timeout /t 2 >nul

REM Lancer serveur sur port 9000
echo Lancement serveur port 9000...
start "SERVEUR" /MIN cmd /k "cd server && set SERVER_HOST=:9000 && signaling-server.exe"
timeout /t 5 >nul

REM Ecrire le port
echo 9000 > server_port.txt

REM Lancer Instance 1
echo Lancement Instance 1...
start "" client\src-tauri\target\release\ghosthanddesk-tauri.exe
timeout /t 4 >nul

REM Lancer Instance 2
echo Lancement Instance 2...
start "" client\src-tauri\target\release\ghosthanddesk-tauri.exe

echo.
echo FAIT !
echo.
echo Instance 1: Copiez le Device ID
echo Instance 2: Connectez-vous avec ce Device ID
echo.
pause
