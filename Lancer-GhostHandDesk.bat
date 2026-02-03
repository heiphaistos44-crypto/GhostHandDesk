@echo off
:: Script de lancement de GhostHandDesk
:: Ce script lance l'application GhostHandDesk compilée

echo.
echo ================================================
echo         🚀 Lancement de GhostHandDesk
echo ================================================
echo.

:: Vérifier si l'exécutable existe
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

:: Lancer l'application
echo ✅ Lancement de l'application...
echo.
start "" "client\src-tauri\target\release\ghosthanddesk-tauri.exe"

echo.
echo ✅ Application lancée !
echo.
echo L'application GhostHandDesk est maintenant en cours d'exécution.
echo Vous pouvez fermer cette fenêtre.
echo.
