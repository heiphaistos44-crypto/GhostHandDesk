@echo off
:: Script de lancement du serveur de signalement GhostHandDesk
:: Ce script lance le serveur Go WebSocket

echo.
echo ================================================
echo    🌐 Serveur de signalement GhostHandDesk
echo ================================================
echo.

:: Vérifier si Go est installé
where go >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: Go n'est pas installé ou pas dans le PATH !
    echo.
    echo Installez Go depuis : https://go.dev/dl/
    echo Puis redémarrez votre terminal.
    echo.
    pause
    exit /b 1
)

echo ✅ Go détecté:
go version
echo.

:: Vérifier les certificats TLS
if not exist "server\certs\server.crt" (
    echo ❌ ERREUR: Certificats TLS manquants !
    echo.
    echo Les certificats doivent être dans server\certs\
    echo.
    pause
    exit /b 1
)

echo ✅ Certificats TLS trouvés
echo.

:: Lancer le serveur
echo 🚀 Démarrage du serveur sur https://localhost:8443
echo.
echo Routes disponibles:
echo   - wss://localhost:8443/ws      (WebSocket)
echo   - https://localhost:8443/health (Health check)
echo   - https://localhost:8443/stats  (Statistiques)
echo.
echo ================================================
echo.

cd server
go run cmd/signaling/main.go

pause
