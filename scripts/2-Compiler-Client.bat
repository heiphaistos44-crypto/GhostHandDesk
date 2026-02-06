@echo off
:: Script simple pour compiler uniquement le client Tauri

echo.
echo ================================================
echo    Compilation du client Tauri
echo ================================================
echo.

:: Vérifier Cargo/Rust
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: Rust/Cargo n'est pas installé !
    echo.
    echo Installez Rust depuis : https://rustup.rs/
    echo.
    pause
    exit /b 1
)

echo ✅ Rust trouvé:
rustc --version
cargo --version
echo.

:: Aller dans le dossier client
cd /d "%~dp0client"

echo Dossier actuel : %CD%
echo.

:: Vérifier que le projet Tauri existe
if not exist "src-tauri\Cargo.toml" (
    echo ❌ ERREUR: Projet Tauri introuvable !
    echo.
    pause
    exit /b 1
)

echo Compilation en cours (cela peut prendre plusieurs minutes)...
echo.
echo Note: Tauri v2 compile en mode release par defaut
echo.
cargo tauri build

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ Erreur lors de la compilation !
    echo.
    pause
    exit /b 1
)

echo.
echo ================================================
echo ✅ SUCCÈS !
echo ================================================
echo.
echo Application compilée :
echo - EXE : src-tauri\target\release\ghosthanddesk-tauri.exe
echo - MSI : src-tauri\target\release\bundle\msi\
echo - NSIS : src-tauri\target\release\bundle\nsis\
echo.

:: Afficher les fichiers
dir /s src-tauri\target\release\*.exe | findstr /v "\.exe\.pdb"
echo.
echo ================================================
pause
