@echo off
:: Script de génération de certificats TLS auto-signés pour GhostHandDesk
:: Usage: generate-certs.bat [output_dir]

setlocal

set OUTPUT_DIR=%~1
if "%OUTPUT_DIR%"=="" set OUTPUT_DIR=..\server\certs

echo.
echo ================================================
echo   Génération de certificats TLS auto-signés
echo ================================================
echo.

:: Créer le dossier de sortie
if not exist "%OUTPUT_DIR%" (
    mkdir "%OUTPUT_DIR%"
    echo ✅ Dossier créé: %OUTPUT_DIR%
)

set CERT_FILE=%OUTPUT_DIR%\cert.pem
set KEY_FILE=%OUTPUT_DIR%\key.pem

:: Vérifier si OpenSSL est installé
where openssl >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: OpenSSL non trouvé dans le PATH
    echo.
    echo Installez OpenSSL depuis:
    echo   - Windows: https://slproweb.com/products/Win32OpenSSL.html
    echo   - Chocolatey: choco install openssl
    echo   - Git for Windows inclut OpenSSL
    echo.
    echo Ou utilisez AUTO_GENERATE_CERTS=true dans le serveur Go
    pause
    exit /b 1
)

:: Générer la clé privée ECDSA (P-256)
echo 🔑 Génération de la clé privée ECDSA...
openssl ecparam -genkey -name prime256v1 -out "%KEY_FILE%"
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Erreur lors de la génération de la clé
    pause
    exit /b 1
)
echo ✅ Clé privée générée: %KEY_FILE%

:: Générer le certificat auto-signé (valide 365 jours)
echo 📜 Génération du certificat auto-signé...
openssl req -new -x509 -key "%KEY_FILE%" -out "%CERT_FILE%" -days 365 ^
    -subj "/C=FR/ST=Dev/L=Dev/O=GhostHandDesk/CN=localhost" ^
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

if %ERRORLEVEL% NEQ 0 (
    echo ❌ Erreur lors de la génération du certificat
    pause
    exit /b 1
)
echo ✅ Certificat généré: %CERT_FILE%

echo.
echo ================================================
echo ✅ CERTIFICATS GÉNÉRÉS AVEC SUCCÈS
echo ================================================
echo.
echo Fichiers créés:
echo   - Certificat: %CERT_FILE%
echo   - Clé privée: %KEY_FILE%
echo.
echo ⚠️  AVERTISSEMENT:
echo   Ces certificats sont auto-signés et destinés
echo   AU DÉVELOPPEMENT UNIQUEMENT.
echo.
echo   En production, utilisez des certificats signés
echo   par une autorité reconnue (Let's Encrypt, etc.)
echo.
echo Configuration serveur:
echo   set CERT_FILE=%CERT_FILE%
echo   set KEY_FILE=%KEY_FILE%
echo   set REQUIRE_TLS=true
echo.
echo ================================================
pause
