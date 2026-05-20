@echo off
chcp 65001 > nul
cls

echo ╔════════════════════════════════════════════════════════════════╗
echo ║         VÉRIFICATION DES PRÉREQUIS - GhostHandDesk             ║
echo ╚════════════════════════════════════════════════════════════════╝
echo.

set ALL_OK=1

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Go...
where go >nul 2>&1
if errorlevel 1 (
    echo ❌ Go n'est pas installé
    echo    Téléchargez depuis: https://go.dev/dl/
    set ALL_OK=0
) else (
    for /f "tokens=3" %%v in ('go version') do set GO_VERSION=%%v
    echo ✅ Go installé: %GO_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Rust...
where rustc >nul 2>&1
if errorlevel 1 (
    echo ❌ Rust n'est pas installé
    echo    Téléchargez depuis: https://rustup.rs/
    set ALL_OK=0
) else (
    for /f "tokens=2" %%v in ('rustc --version') do set RUST_VERSION=%%v
    echo ✅ Rust installé: %RUST_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Cargo...
where cargo >nul 2>&1
if errorlevel 1 (
    echo ❌ Cargo n'est pas installé
    echo    Installez Rust: https://rustup.rs/
    set ALL_OK=0
) else (
    for /f "tokens=2" %%v in ('cargo --version') do set CARGO_VERSION=%%v
    echo ✅ Cargo installé: %CARGO_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Node.js...
where node >nul 2>&1
if errorlevel 1 (
    echo ❌ Node.js n'est pas installé
    echo    Téléchargez depuis: https://nodejs.org/
    set ALL_OK=0
) else (
    for /f %%v in ('node --version') do set NODE_VERSION=%%v
    echo ✅ Node.js installé: %NODE_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de npm...
where npm >nul 2>&1
if errorlevel 1 (
    echo ❌ npm n'est pas installé
    echo    Réinstallez Node.js: https://nodejs.org/
    set ALL_OK=0
) else (
    for /f %%v in ('npm --version') do set NPM_VERSION=%%v
    echo ✅ npm installé: v%NPM_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Tauri CLI...
where cargo >nul 2>&1
if not errorlevel 1 (
    cargo install --list | findstr "tauri-cli" >nul 2>&1
    if errorlevel 1 (
        echo ⚠️  Tauri CLI n'est pas installé
        echo    Installez avec: cargo install tauri-cli
        echo    (Optionnel, mais recommandé)
    ) else (
        echo ✅ Tauri CLI installé
    )
) else (
    echo ⚠️  Impossible de vérifier Tauri CLI (Cargo non installé)
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo 🔍 Vérification de Git (optionnel)...
where git >nul 2>&1
if errorlevel 1 (
    echo ⚠️  Git n'est pas installé (optionnel)
    echo    Téléchargez depuis: https://git-scm.com/
) else (
    for /f "tokens=3" %%v in ('git --version') do set GIT_VERSION=%%v
    echo ✅ Git installé: %GIT_VERSION%
)
echo.

REM ═══════════════════════════════════════════════════════════════
echo.
echo ═══════════════════════════════════════════════════════════════

if %ALL_OK%==1 (
    echo.
    echo ✅✅✅ TOUS LES PRÉREQUIS SONT INSTALLÉS ! ✅✅✅
    echo.
    echo 🚀 Vous pouvez maintenant lancer le build:
    echo.
    echo    .\BUILD-PORTABLE-v0.2.1.bat
    echo.
) else (
    echo.
    echo ❌❌❌ CERTAINS PRÉREQUIS MANQUENT ❌❌❌
    echo.
    echo 📋 Installez les outils manquants puis relancez ce script
    echo.
)

echo ═══════════════════════════════════════════════════════════════
echo.

REM ═══════════════════════════════════════════════════════════════
echo 📊 Informations système additionnelles:
echo.

REM Vérifier l'espace disque disponible
for /f "tokens=3" %%a in ('dir /-c ^| find "bytes free"') do set FREE_SPACE=%%a
echo    💾 Espace disque libre: %FREE_SPACE% bytes

REM Vérifier la version de Windows
for /f "tokens=4-7 delims=[.] " %%i in ('ver') do set WIN_VERSION=%%i.%%j
echo    🪟 Windows: %WIN_VERSION%

REM Vérifier la version de PowerShell (si disponible)
where powershell >nul 2>&1
if not errorlevel 1 (
    for /f "tokens=*" %%v in ('powershell -Command "$PSVersionTable.PSVersion.Major"') do set PS_VERSION=%%v
    echo    ⚡ PowerShell: v%PS_VERSION%
)

echo.
echo ═══════════════════════════════════════════════════════════════
echo.
pause
