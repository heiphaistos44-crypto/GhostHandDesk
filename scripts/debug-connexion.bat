@echo off
cls
echo ========================================
echo   Debug - Test de Connexion
echo ========================================
echo.

set "ROOT_DIR=%~dp0"
cd /d "%ROOT_DIR%"

REM Arrêter tous les processus existants
echo Nettoyage des processus existants...
taskkill /F /IM signaling-server.exe 2>nul
taskkill /F /IM ghosthanddesk-tauri.exe 2>nul
timeout /t 2 /nobreak >nul

echo.
echo [1/3] Demarrage du serveur avec logs visibles...
start "SERVEUR - NE PAS FERMER" cmd /k "cd /d %ROOT_DIR%server && signaling-server.exe"

echo Attente du serveur...
timeout /t 5 /nobreak >nul

echo.
echo [2/3] Lancement Instance 1 (PARTAGE SON ECRAN)...
start "Instance 1 - PARTAGE" "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe"

timeout /t 3 /nobreak >nul

echo.
echo [3/3] Lancement Instance 2 (PREND LE CONTROLE)...
start "Instance 2 - CONTROLE" "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe"

echo.
echo ========================================
echo   INSTRUCTIONS DE TEST
echo ========================================
echo.
echo IMPORTANT: Gardez la fenetre SERVEUR ouverte pour voir les logs !
echo.
echo 1. Dans Instance 1: Notez le Device ID (ex: GHD-abc123)
echo 2. Dans Instance 2: Entrez ce Device ID et cliquez "Se connecter"
echo 3. Surveillez la fenetre SERVEUR pour voir les messages
echo 4. Dans Instance 1: Acceptez la connexion dans le popup
echo.
echo Si ca bloque sur "Connexion en cours":
echo   - Verifiez les logs du SERVEUR
echo   - Verifiez que les deux Device IDs sont differents
echo   - Appuyez sur F12 dans l'app pour ouvrir les DevTools
echo.
pause
