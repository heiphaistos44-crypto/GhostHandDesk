@echo off
set "ROOT_DIR=%~dp0"
start "GhostHandDesk" "%ROOT_DIR%client\src-tauri\target\release\ghosthanddesk-tauri.exe"
echo Application lancee !
echo.
echo Pour lancer une autre instance, relancez ce script.
timeout /t 2 /nobreak >nul
