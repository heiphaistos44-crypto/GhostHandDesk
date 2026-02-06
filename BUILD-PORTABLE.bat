@echo off
:: Script pour créer le package portable
:: Utilise les fichiers déjà compilés

echo.
echo ================================================
echo    Création du package portable
echo ================================================
echo.

cd /d "%~dp0"

:: Vérifier que les exécutables existent
echo Vérification des fichiers...
echo.

set SERVER_EXE=server\signaling-server.exe
set CLIENT_EXE=client\src-tauri\target\release\ghosthanddesk-tauri.exe

if not exist "%SERVER_EXE%" (
    echo ❌ ERREUR: %SERVER_EXE% introuvable !
    echo.
    echo Compilez d'abord le serveur avec : 1-Compiler-Serveur.bat
    echo.
    pause
    exit /b 1
)
echo ✅ Serveur trouvé : %SERVER_EXE%

if not exist "%CLIENT_EXE%" (
    echo ❌ ERREUR: %CLIENT_EXE% introuvable !
    echo.
    echo Compilez d'abord le client avec : 2-Compiler-Client.bat
    echo.
    pause
    exit /b 1
)
echo ✅ Client trouvé : %CLIENT_EXE%
echo.

:: Créer le dossier portable
set PORTABLE_DIR=GhostHandDesk-Portable
echo Création du dossier : %PORTABLE_DIR%
if exist "%PORTABLE_DIR%" (
    echo   Suppression de l'ancien dossier...
    rmdir /s /q "%PORTABLE_DIR%"
)
mkdir "%PORTABLE_DIR%"
echo.

:: Copier les fichiers
echo Copie des fichiers...
copy "%CLIENT_EXE%" "%PORTABLE_DIR%\ghosthanddesk-tauri.exe" >nul
echo ✅ ghosthanddesk-tauri.exe copié

copy "%SERVER_EXE%" "%PORTABLE_DIR%\signaling-server.exe" >nul
echo ✅ signaling-server.exe copié

:: Créer le dossier certs (vide)
mkdir "%PORTABLE_DIR%\certs"
echo ✅ Dossier certs créé (sera rempli auto au démarrage)
echo.

:: Créer le README
echo Création du README.txt...
(
echo ========================================
echo    GhostHandDesk - Version Portable
echo ========================================
echo.
echo LANCEMENT :
echo   Double-cliquez sur ghosthanddesk-tauri.exe
echo.
echo CARACTERISTIQUES :
echo   - 100%% portable, aucune installation
echo   - Serveur embarque, lance automatiquement
echo   - Certificats generes automatiquement
echo   - Aucune trace sur le PC
echo.
echo UTILISATION :
echo.
echo 1. PC Principal :
echo    - Lancez ghosthanddesk-tauri.exe
echo    - Notez le Device ID affiche
echo.
echo 2. VM ou PC distant :
echo    - Copiez ce dossier sur la machine
echo    - Lancez ghosthanddesk-tauri.exe
echo    - Notez le Device ID
echo.
echo 3. Connexion :
echo    - Sur le PC principal, entrez le Device ID de la VM
echo    - Laissez le mot de passe vide ^(optionnel^)
echo    - Cliquez "Connect"
echo.
echo SUPPRESSION :
echo   Supprimez simplement ce dossier.
echo   Aucune trace ne restera sur votre PC.
echo.
echo ========================================
echo Version : 0.1.0 Portable
echo Date : %DATE%
echo ========================================
) > "%PORTABLE_DIR%\README.txt"
echo ✅ README.txt créé
echo.

:: Afficher le résumé
echo ================================================
echo ✅ PACKAGE PORTABLE CRÉÉ !
echo ================================================
echo.
echo Dossier : %CD%\%PORTABLE_DIR%
echo.
echo Contenu :
dir /b "%PORTABLE_DIR%"
echo.
echo Taille totale :
dir "%PORTABLE_DIR%\*.exe"
echo.

:: Optionnel : créer une archive ZIP
where 7z >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo Création de l'archive ZIP...
    7z a -tzip GhostHandDesk-Portable.zip "%PORTABLE_DIR%\" >nul
    echo ✅ Archive créée : GhostHandDesk-Portable.zip
    echo.
) else (
    echo Note: 7-Zip non trouvé. Archive ZIP non créée.
    echo Pour créer une archive, installez 7-Zip ou utilisez WinRAR.
    echo.
)

echo ================================================
echo.
echo Pour distribuer l'application :
echo - Copiez le dossier "%PORTABLE_DIR%" sur une clé USB
echo - Ou envoyez le fichier GhostHandDesk-Portable.zip
echo.
echo IMPORTANT : Vos clients n'ont besoin d'AUCUNE installation !
echo L'application fonctionne directement.
echo.
echo ================================================
pause
