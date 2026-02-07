@echo off
echo ========================================
echo    GhostHandDesk - Lancement Complet
echo ========================================
echo.

REM Sauvegarder le répertoire actuel
set "ROOT_DIR=%CD%"

echo [1/3] Demarrage du serveur de signalement...
start "Serveur Signalement" cmd /c "cd /d "%ROOT_DIR%\server" && signaling-server.exe"
timeout /t 3 /nobreak >nul

echo [2/3] Demarrage de l'application Tauri (avec Vite integre)...
echo Note: Tauri va automatiquement lancer le serveur Vite
echo.
cd /d "%ROOT_DIR%\client\src-tauri"
cargo tauri dev

echo.
echo Application terminee.
pause
