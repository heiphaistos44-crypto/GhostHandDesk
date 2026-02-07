@echo off
:: Script de lancement GhostHandDesk v0.2.0
:: Lance automatiquement le serveur puis le client

cd /d "%~dp0"

echo.
echo ==============================================
echo    GhostHandDesk v0.2.0 - Lancement
echo ==============================================
echo.

:: Vérifier si le serveur est déjà lancé
tasklist /FI "IMAGENAME eq signaling-server.exe" 2>NUL | find /I /N "signaling-server.exe">NUL
if "%ERRORLEVEL%"=="0" (
    echo [INFO] Serveur de signaling déjà actif
) else (
    echo [1/2] Démarrage du serveur de signaling...

    :: Créer un script temporaire avec les variables d'environnement
    echo @echo off > _start_server.bat
    echo set REQUIRE_TLS=false >> _start_server.bat
    echo set PORT=9000 >> _start_server.bat
    echo set DISABLE_ORIGIN_CHECK=true >> _start_server.bat
    echo signaling-server.exe >> _start_server.bat

    :: Lancer le serveur en arrière-plan avec le script
    start "GhostHandDesk Server" /MIN _start_server.bat

    :: Attendre que le serveur démarre (3 secondes)
    timeout /t 3 /nobreak >nul

    echo    ✅ Serveur démarré (port 9000)
)

echo.
echo [2/2] Lancement de l'application...
echo    ✅ Interface Tauri
echo.
echo ==============================================
echo    Application prête !
echo ==============================================
echo.

:: Lancer l'application Tauri
start "" ghosthanddesk-tauri.exe

:: Attendre la fin de l'application
echo Appuyez sur une touche pour arrêter le serveur...
pause >nul

:: Tuer le serveur quand on ferme
echo.
echo Arrêt du serveur...
taskkill /F /IM signaling-server.exe >nul 2>&1
del /F /Q _start_server.bat 2>nul
echo ✅ Serveur arrêté
echo.
