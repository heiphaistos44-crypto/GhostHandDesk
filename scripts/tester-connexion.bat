@echo off
echo ========================================
echo   Test de Connexion GhostHandDesk
echo ========================================
echo.

REM Vérifier que le serveur tourne
echo Verification du serveur de signalement...
netstat -an | findstr "9080" >nul
if errorlevel 1 (
    echo [ERREUR] Le serveur n'est pas lance !
    echo Lancez d'abord: server\signaling-server.exe
    pause
    exit /b 1
)
echo [OK] Serveur actif sur le port 9080
echo.

echo Lancement de deux instances de l'application...
echo.
echo Instance 1: Sera le client CONTROLE (celui qui partage son ecran)
start "GhostHandDesk - Instance 1 (Controle)" "%~dp0client\src-tauri\target\release\ghosthanddesk-tauri.exe"

timeout /t 2 /nobreak >nul

echo Instance 2: Sera le client CONTROLEUR (celui qui prend le controle)
start "GhostHandDesk - Instance 2 (Controleur)" "%~dp0client\src-tauri\target\release\ghosthanddesk-tauri.exe"

echo.
echo ========================================
echo   Instructions de Test
echo ========================================
echo.
echo 1. Dans l'Instance 1, copiez le Device ID affiche en haut
echo 2. Dans l'Instance 2, cliquez sur "Connect" et collez le Device ID
echo 3. Dans l'Instance 1, un popup apparaitra - cliquez "Accepter"
echo 4. La connexion WebRTC devrait s'etablir !
echo.
echo Note: Le serveur doit rester actif pendant tout le test.
echo.
pause
