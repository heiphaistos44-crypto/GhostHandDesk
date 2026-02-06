@echo off
:: Script de compilation GhostHandDesk Portable
:: Crée une application 100% portable sans trace sur le PC

echo.
echo ================================================
echo    📦 Compilation GhostHandDesk Portable
echo ================================================
echo.

:: Vérifier Go
where go >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: Go n'est pas installé !
    echo.
    echo Installez Go depuis : https://go.dev/dl/
    echo Puis relancez ce script.
    echo.
    pause
    exit /b 1
)

echo ✅ Go détecté:
go version
echo.

:: Vérifier Rust/Cargo
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: Rust/Cargo n'est pas installé !
    echo.
    echo Installez Rust depuis : https://rustup.rs/
    echo Puis relancez ce script.
    echo.
    pause
    exit /b 1
)

echo ✅ Rust détecté:
rustc --version
echo.

:: Étape 1 : Compiler le serveur Go
echo ================================================
echo [1/3] Compilation du serveur Go...
echo ================================================
echo.

echo Compilation depuis : %CD%\server
cd /d "%~dp0server"

if not exist "cmd\signaling\main.go" (
    echo ❌ ERREUR: Fichier cmd\signaling\main.go introuvable !
    echo Chemin actuel: %CD%
    cd ..
    pause
    exit /b 1
)

go build -ldflags="-s -w" -o signaling-server.exe cmd/signaling/main.go

if %ERRORLEVEL% NEQ 0 (
    echo ❌ Erreur lors de la compilation du serveur Go
    cd ..
    pause
    exit /b 1
)

echo ✅ Serveur Go compilé : %CD%\signaling-server.exe
cd ..
echo.

:: Étape 2 : Compiler l'application Tauri
echo ================================================
echo [2/3] Compilation de l'application Tauri...
echo ================================================
echo.

cd /d "%~dp0client"

if not exist "src-tauri\Cargo.toml" (
    echo ❌ ERREUR: Projet Tauri introuvable !
    echo Chemin actuel: %CD%
    cd ..
    pause
    exit /b 1
)

cargo tauri build --release

if %ERRORLEVEL% NEQ 0 (
    echo ❌ Erreur lors de la compilation Tauri
    cd ..
    pause
    exit /b 1
)

echo ✅ Application Tauri compilée
cd ..
echo.

:: Étape 3 : Créer le package portable
echo ================================================
echo [3/3] Création du package portable...
echo ================================================
echo.

:: Retourner au dossier racine
cd /d "%~dp0"

:: Créer le dossier portable
set PORTABLE_DIR=%~dp0GhostHandDesk-Portable
if exist "%PORTABLE_DIR%" rmdir /s /q "%PORTABLE_DIR%"
mkdir "%PORTABLE_DIR%"

:: Copier l'exécutable Tauri
if not exist "client\src-tauri\target\release\ghosthanddesk-tauri.exe" (
    echo ❌ ERREUR: ghosthanddesk-tauri.exe introuvable !
    pause
    exit /b 1
)
copy "client\src-tauri\target\release\ghosthanddesk-tauri.exe" "%PORTABLE_DIR%\" >nul

:: Copier le serveur Go
if not exist "server\signaling-server.exe" (
    echo ❌ ERREUR: signaling-server.exe introuvable !
    pause
    exit /b 1
)
copy "server\signaling-server.exe" "%PORTABLE_DIR%\" >nul

:: Créer le dossier certs (vide, sera généré auto)
mkdir "%PORTABLE_DIR%\certs"

:: Créer un fichier README
echo # GhostHandDesk - Version Portable > "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo Cette application est 100%% portable et ne laisse aucune trace sur votre PC. >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo ## Lancement : >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo 1. Double-cliquez sur ghosthanddesk-tauri.exe >> "%PORTABLE_DIR%\README.txt"
echo 2. Le serveur se lance automatiquement >> "%PORTABLE_DIR%\README.txt"
echo 3. Les certificats sont generes automatiquement au premier lancement >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo ## Caracteristiques : >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo - Aucune installation requise >> "%PORTABLE_DIR%\README.txt"
echo - Aucun fichier dans AppData >> "%PORTABLE_DIR%\README.txt"
echo - Aucune cle de registre >> "%PORTABLE_DIR%\README.txt"
echo - Serveur de signalement embarque >> "%PORTABLE_DIR%\README.txt"
echo - Certificats TLS auto-generes >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo ## Suppression : >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo Supprimez simplement le dossier GhostHandDesk-Portable. >> "%PORTABLE_DIR%\README.txt"
echo Aucune trace ne restera sur votre PC. >> "%PORTABLE_DIR%\README.txt"

echo ✅ Package portable créé : %PORTABLE_DIR%\
echo.

:: Afficher le contenu
echo Contenu du package :
dir /b %PORTABLE_DIR%
echo.

:: Créer une archive ZIP (si 7-Zip est disponible)
where 7z >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo Création de l'archive ZIP...
    7z a -tzip GhostHandDesk-Portable.zip %PORTABLE_DIR%\ >nul
    echo ✅ Archive créée : GhostHandDesk-Portable.zip
    echo.
)

echo ================================================
echo ✅ COMPILATION TERMINÉE !
echo ================================================
echo.
echo 📦 Dossier portable : %PORTABLE_DIR%\
echo 📱 Exécutable : %PORTABLE_DIR%\ghosthanddesk-tauri.exe
echo 🌐 Serveur : %PORTABLE_DIR%\signaling-server.exe
echo.
echo Pour distribuer l'application :
echo - Copiez le dossier %PORTABLE_DIR% sur une clé USB
echo - Ou utilisez l'archive GhostHandDesk-Portable.zip
echo.
echo ================================================
pause
