@echo off
:: Script de lancement complet de GhostHandDesk
:: Ce script lance le serveur de signalisation ET l'application client

echo.
echo ================================================
echo         🚀 Lancement de GhostHandDesk
echo ================================================
echo.

cd /d "%~dp0"

:: Vérifier si le serveur existe
if not exist "server\signaling-server.exe" (
    echo ❌ ERREUR: Le serveur signaling-server.exe n'a pas été trouvé !
    echo.
    echo Compilez d'abord le serveur avec :
    echo   cd server
    echo   go build -o signaling-server.exe ./cmd/signaling
    echo.
    pause
    exit /b 1
)

:: Vérifier si l'exécutable client existe
if not exist "client\src-tauri\target\release\ghosthanddesk-tauri.exe" (
    echo ❌ ERREUR: Le fichier ghosthanddesk-tauri.exe n'a pas été trouvé !
    echo.
    echo Vous devez d'abord compiler l'application avec :
    echo   cd client
    echo   cargo tauri build
    echo.
    pause
    exit /b 1
)

:: Lancer le serveur de signalisation en arrière-plan
echo ✅ Lancement du serveur de signalisation...
echo.
start "GhostHandDesk Server" /MIN "server\signaling-server.exe"

:: Attendre 2 secondes que le serveur démarre
timeout /t 2 /nobreak >nul

:: Lancer l'application client
echo ✅ Lancement de l'application client...
echo.
start "GhostHandDesk Client" "client\src-tauri\target\release\ghosthanddesk-tauri.exe"

echo.
echo ================================================
echo ✅ GhostHandDesk est maintenant lancé !
echo ================================================
echo.
echo Le serveur fonctionne en arrière-plan (fenêtre minimisée)
echo L'application cliente est ouverte
echo.
echo Pour arrêter complètement l'application :
echo   - Fermez la fenêtre de l'application
echo   - Fermez la fenêtre du serveur (minimisée)
echo.
echo Vous pouvez fermer cette fenêtre.
echo.
pause
