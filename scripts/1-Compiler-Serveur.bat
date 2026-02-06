@echo off
:: Script simple pour compiler uniquement le serveur Go

echo.
echo ================================================
echo    Compilation du serveur Go
echo ================================================
echo.

:: Vérifier Go
where go >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ ERREUR: Go n'est pas installé ou pas dans le PATH !
    echo.
    echo Solutions :
    echo 1. Installez Go depuis : https://go.dev/dl/
    echo 2. Redémarrez ce terminal après installation
    echo 3. Ou ajoutez Go au PATH manuellement
    echo.
    pause
    exit /b 1
)

echo ✅ Go trouvé:
go version
echo.

:: Aller dans le dossier server
cd /d "%~dp0server"

echo Dossier actuel : %CD%
echo.

:: Vérifier que le fichier main.go existe
if not exist "cmd\signaling\main.go" (
    echo ❌ ERREUR: cmd\signaling\main.go introuvable !
    echo.
    dir /s main.go
    echo.
    pause
    exit /b 1
)

:: Télécharger les dépendances Go et générer go.sum
echo Téléchargement des dépendances Go...
go mod tidy

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ Erreur lors du téléchargement des dépendances !
    echo.
    pause
    exit /b 1
)
echo ✅ Dépendances téléchargées et go.sum généré
echo.

echo Compilation en cours...
go build -ldflags="-s -w" -o signaling-server.exe cmd/signaling/main.go

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
echo Serveur compilé : %CD%\signaling-server.exe
echo.

:: Afficher la taille du fichier
dir signaling-server.exe
echo.
echo ================================================
pause
