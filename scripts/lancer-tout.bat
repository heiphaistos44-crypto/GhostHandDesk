@echo off
cls
echo ========================================
echo    GhostHandDesk - Lancement Complet
echo ========================================
echo.

REM Sauvegarder le répertoire racine
set "ROOT_DIR=%~dp0"
cd /d "%ROOT_DIR%"

echo [1/3] Demarrage du serveur de signalement...
start "Serveur de Signalement" /MIN cmd /c "%ROOT_DIR%server\signaling-server.exe"

echo Attente du demarrage du serveur (5 secondes)...
timeout /t 5 /nobreak >nul

echo [2/3] Verification que le serveur est actif...
netstat -an | findstr "9080" >nul
if errorlevel 1 (
    echo [ERREUR] Le serveur n'a pas demarre correctement !
    echo Verifiez que le port 9080 n'est pas deja utilise.
    pause
    exit /b 1
)
echo [OK] Serveur actif sur le port 9080
echo.

echo [3/3] Lancement de deux instances de l'application...
echo.

echo Instance 1 (CONTROLE - celui qui partage son ecran)...
start "GhostHandDesk - Instance 1" "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe"

timeout /t 3 /nobreak >nul

echo Instance 2 (CONTROLEUR - celui qui prend le controle)...
start "GhostHandDesk - Instance 2" "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe"

echo.
echo ========================================
echo   Instructions de Test
echo ========================================
echo.
echo ETAPE 1: Dans la fenetre "Instance 1"
echo    - Copiez le Device ID affiche en haut (bouton copier)
echo.
echo ETAPE 2: Dans la fenetre "Instance 2"
echo    - Cliquez sur le bouton "Se connecter a un appareil"
echo    - Collez le Device ID de l'Instance 1
echo    - Cliquez "Connecter"
echo.
echo ETAPE 3: Dans la fenetre "Instance 1"
echo    - Un popup "Demande de Connexion" va apparaitre
echo    - Cliquez sur "Accepter"
echo.
echo ETAPE 4: Resultat attendu
echo    - L'Instance 2 devrait afficher l'ecran de l'Instance 1
echo    - Vous pouvez controler la souris/clavier a distance
echo.
echo ========================================
echo.
echo Pour arreter tout:
echo    - Fermez les deux fenetres de l'application
echo    - Fermez la fenetre "Serveur de Signalement"
echo.
pause
