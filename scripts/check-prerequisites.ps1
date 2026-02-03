# Script de Vérification des Prérequis - GhostHandDesk
# Vérifie que tous les outils nécessaires sont installés

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  GhostHandDesk - Vérification Système" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

$allOk = $true

# Fonction helper pour vérifier une commande
function Test-Command {
    param (
        [string]$CommandName,
        [string]$DisplayName,
        [string]$MinVersion,
        [string]$InstallCmd
    )

    Write-Host "Vérification $DisplayName..." -NoNewline

    $cmd = Get-Command $CommandName -ErrorAction SilentlyContinue

    if ($cmd) {
        Write-Host " ✅" -ForegroundColor Green

        # Afficher version si disponible
        try {
            $version = & $CommandName --version 2>&1 | Select-Object -First 1
            Write-Host "  Version: $version" -ForegroundColor Gray
        } catch {
            Write-Host "  Installé (version non détectable)" -ForegroundColor Gray
        }

        return $true
    } else {
        Write-Host " ❌" -ForegroundColor Red
        Write-Host "  Non installé. Installation:" -ForegroundColor Yellow
        Write-Host "  $InstallCmd" -ForegroundColor Yellow
        return $false
    }
}

Write-Host "=== Outils Système ===" -ForegroundColor Yellow

# Vérifier Go
if (-not (Test-Command -CommandName "go" -DisplayName "Go (Golang)" -MinVersion "1.21" -InstallCmd "choco install golang -y")) {
    $allOk = $false
}

# Vérifier FFmpeg
if (-not (Test-Command -CommandName "ffmpeg" -DisplayName "FFmpeg" -MinVersion "4.0" -InstallCmd "choco install ffmpeg -y")) {
    Write-Host "  ⚠️  Non critique - Le client utilisera l'encodeur JPEG en fallback" -ForegroundColor Yellow
}

# Vérifier OpenSSL
if (-not (Test-Command -CommandName "openssl" -DisplayName "OpenSSL" -MinVersion "1.1" -InstallCmd "choco install openssl -y")) {
    Write-Host "  ⚠️  Nécessaire pour générer les certificats TLS" -ForegroundColor Yellow
}

# Vérifier Rust
if (-not (Test-Command -CommandName "cargo" -DisplayName "Rust (cargo)" -MinVersion "1.70" -InstallCmd "https://rustup.rs")) {
    $allOk = $false
}

# Vérifier Node.js
if (-not (Test-Command -CommandName "node" -DisplayName "Node.js" -MinVersion "18.0" -InstallCmd "choco install nodejs -y")) {
    $allOk = $false
}

# Vérifier npm
if (-not (Test-Command -CommandName "npm" -DisplayName "npm" -MinVersion "9.0" -InstallCmd "Inclus avec Node.js")) {
    $allOk = $false
}

Write-Host "`n=== Outils Rust ===" -ForegroundColor Yellow

# Vérifier Tauri CLI
$tauriInstalled = cargo install --list | Select-String "tauri-cli"
if ($tauriInstalled) {
    Write-Host "Vérification Tauri CLI... ✅" -ForegroundColor Green
    Write-Host "  Version: $tauriInstalled" -ForegroundColor Gray
} else {
    Write-Host "Vérification Tauri CLI... ❌" -ForegroundColor Red
    Write-Host "  Installation: cargo install tauri-cli" -ForegroundColor Yellow
    $allOk = $false
}

Write-Host "`n=== Configuration Réseau ===" -ForegroundColor Yellow

# Vérifier port 8443 disponible
Write-Host "Vérification port 8443..." -NoNewline
$port8443 = Get-NetTCPConnection -LocalPort 8443 -ErrorAction SilentlyContinue
if ($port8443) {
    Write-Host " ⚠️" -ForegroundColor Yellow
    Write-Host "  Port 8443 déjà utilisé (PID: $($port8443.OwningProcess))" -ForegroundColor Yellow
    Write-Host "  Arrêter avec: taskkill /PID $($port8443.OwningProcess) /F" -ForegroundColor Yellow
} else {
    Write-Host " ✅" -ForegroundColor Green
    Write-Host "  Port disponible" -ForegroundColor Gray
}

# Vérifier port 5173 (Vite)
Write-Host "Vérification port 5173 (Vite)..." -NoNewline
$port5173 = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
if ($port5173) {
    Write-Host " ⚠️" -ForegroundColor Yellow
    Write-Host "  Port 5173 déjà utilisé" -ForegroundColor Yellow
} else {
    Write-Host " ✅" -ForegroundColor Green
}

Write-Host "`n=== Dépendances Projet ===" -ForegroundColor Yellow

# Vérifier server/go.mod
Write-Host "Vérification server/go.mod..." -NoNewline
if (Test-Path "server/go.mod") {
    Write-Host " ✅" -ForegroundColor Green
} else {
    Write-Host " ❌" -ForegroundColor Red
    Write-Host "  Exécuter: cd server && go mod init ghosthanddesk/server" -ForegroundColor Yellow
    $allOk = $false
}

# Vérifier client/Cargo.toml
Write-Host "Vérification client/Cargo.toml..." -NoNewline
if (Test-Path "client/Cargo.toml") {
    Write-Host " ✅" -ForegroundColor Green
} else {
    Write-Host " ❌" -ForegroundColor Red
    $allOk = $false
}

# Vérifier client/ui/package.json
Write-Host "Vérification client/ui/package.json..." -NoNewline
if (Test-Path "client/ui/package.json") {
    Write-Host " ✅" -ForegroundColor Green

    # Vérifier node_modules
    Write-Host "Vérification node_modules..." -NoNewline
    if (Test-Path "client/ui/node_modules") {
        Write-Host " ✅" -ForegroundColor Green
    } else {
        Write-Host " ⚠️" -ForegroundColor Yellow
        Write-Host "  Exécuter: cd client/ui && npm install" -ForegroundColor Yellow
    }
} else {
    Write-Host " ❌" -ForegroundColor Red
    $allOk = $false
}

# Vérifier certificats TLS
Write-Host "Vérification certificats TLS..." -NoNewline
if ((Test-Path "server/certs/server.crt") -and (Test-Path "server/certs/server.key")) {
    Write-Host " ✅" -ForegroundColor Green
} else {
    Write-Host " ⚠️" -ForegroundColor Yellow
    Write-Host "  Générer avec:" -ForegroundColor Yellow
    Write-Host "  mkdir server\certs" -ForegroundColor Yellow
    Write-Host "  openssl req -x509 -newkey rsa:4096 -nodes -keyout server\certs\server.key -out server\certs\server.crt -days 365 -subj '/CN=localhost'" -ForegroundColor Yellow
}

Write-Host "`n=== Capacités Système ===" -ForegroundColor Yellow

# Vérifier RAM disponible
$ram = Get-CimInstance Win32_OperatingSystem
$freeRamGB = [math]::Round($ram.FreePhysicalMemory / 1MB, 2)
$totalRamGB = [math]::Round($ram.TotalVisibleMemorySize / 1MB, 2)

Write-Host "RAM disponible: $freeRamGB GB / $totalRamGB GB" -ForegroundColor Gray
if ($freeRamGB -lt 2) {
    Write-Host "  ⚠️  Moins de 2 GB disponible - Performance réduite possible" -ForegroundColor Yellow
}

# Vérifier CPU
$cpu = Get-CimInstance Win32_Processor
Write-Host "CPU: $($cpu.Name)" -ForegroundColor Gray
Write-Host "Cœurs: $($cpu.NumberOfCores) / Threads: $($cpu.NumberOfLogicalProcessors)" -ForegroundColor Gray

# Vérifier espace disque
$disk = Get-PSDrive C
$freeSpaceGB = [math]::Round($disk.Free / 1GB, 2)
Write-Host "Espace disque C: $freeSpaceGB GB disponible" -ForegroundColor Gray
if ($freeSpaceGB -lt 5) {
    Write-Host "  ⚠️  Moins de 5 GB disponible - Compilation difficile" -ForegroundColor Yellow
}

# Résumé final
Write-Host "`n========================================" -ForegroundColor Cyan

if ($allOk) {
    Write-Host "✅ Tous les prérequis critiques sont satisfaits !" -ForegroundColor Green
    Write-Host "`nVous pouvez lancer les tests E2E avec:" -ForegroundColor White
    Write-Host "  .\scripts\run-e2e-tests.ps1" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "❌ Certains prérequis manquent" -ForegroundColor Red
    Write-Host "`nInstallez les outils manquants puis relancez ce script." -ForegroundColor Yellow
    exit 1
}
