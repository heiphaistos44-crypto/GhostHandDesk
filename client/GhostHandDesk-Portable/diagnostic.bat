@echo off
chcp 65001 > nul
echo ============================================
echo    Diagnostic GhostHandDesk
echo ============================================
echo.

echo [1/4] Vérification du serveur...
curl -s http://localhost:8080/health
echo.
echo.

echo [2/4] Test de connexion WebSocket...
echo (Ceci va échouer, c'est normal pour le diagnostic)
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Version: 13" http://localhost:8080/ws 2>&1 | findstr /C:"HTTP" /C:"Upgrade"
echo.
echo.

echo [3/4] Vérification des processus...
tasklist | findstr /I "signaling-server ghosthanddesk"
echo.
echo.

echo [4/4] Vérification du port 8080...
netstat -an | findstr ":8080"
echo.
echo.

echo ============================================
echo Diagnostic terminé
echo ============================================
pause
