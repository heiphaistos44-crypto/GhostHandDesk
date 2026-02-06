# Guide de Tests End-to-End - GhostHandDesk

**Date:** 2026-01-31
**Version:** 1.0
**Statut:** Prêt pour exécution (nécessite Go + FFmpeg)

## 📋 Vue d'ensemble

Ce guide décrit les tests end-to-end complets pour valider que GhostHandDesk fonctionne correctement dans des conditions réelles.

**Durée estimée:** 30-45 minutes
**Prérequis:** Go 1.21+, FFmpeg, 2 machines ou VMs

---

## ✅ Prérequis Système

### Installation Go (Windows)

```powershell
# Via Chocolatey (recommandé)
choco install golang -y

# Vérification
go version  # Doit afficher go1.21+ ou supérieur
```

**Alternative manuelle:**
1. Télécharger depuis https://go.dev/dl/
2. Exécuter l'installateur MSI
3. Redémarrer le terminal
4. Vérifier avec `go version`

### Installation FFmpeg (Windows)

```powershell
# Via Chocolatey (recommandé)
choco install ffmpeg -y

# Vérification
ffmpeg -version  # Doit afficher version FFmpeg
```

**Alternative manuelle:**
1. Télécharger depuis https://ffmpeg.org/download.html
2. Extraire dans `C:\ffmpeg`
3. Ajouter `C:\ffmpeg\bin` au PATH
4. Redémarrer le terminal

### Vérification Complète

```powershell
# Script de vérification automatique
cd Documents/GhostHandDesk
.\scripts\check-prerequisites.ps1
```

---

## 🧪 Scénario 1 : Test de Connexion Locale (LAN)

### Objectif
Valider la connexion WebRTC P2P entre 2 clients sur le même réseau local.

### Étapes

#### 1. Démarrer le Serveur de Signalement

```bash
# Terminal 1
cd Documents/GhostHandDesk/server

# Générer certificats TLS (première fois seulement)
mkdir -p certs
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout certs/server.key \
  -out certs/server.crt \
  -days 365 \
  -subj "/CN=localhost"

# Lancer le serveur
go run cmd/signaling/main.go
```

**✅ Validation Serveur:**
```
==============================================
🚀 GhostHandDesk v0.1.0
==============================================
[MAIN] Configuration chargée: Host=:8443
[MAIN] Hub de signalement démarré
[MAIN] Serveur de signalement démarré sur :8443
[MAIN] Routes disponibles:
  - wss://localhost:8443/ws (WebSocket)
  - https://localhost:8443/health
  - https://localhost:8443/stats
```

**🐛 Troubleshooting:**
- Si erreur "port already in use": `netstat -ano | findstr :8443` puis `taskkill /PID <PID> /F`
- Si erreur certificat: Supprimer `certs/` et régénérer

#### 2. Test Santé du Serveur

```bash
# Terminal 2 (nouveau)
curl -k https://localhost:8443/health
```

**✅ Résultat Attendu:**
```json
{
  "status": "healthy",
  "clients": 0
}
```

#### 3. Installer Dépendances Frontend

```bash
# Terminal 2
cd Documents/GhostHandDesk/client/ui

# Installer npm packages (première fois seulement)
npm install
```

**✅ Validation:**
```
added 124 packages in 15s
```

#### 4. Lancer Client Host (Machine A)

```bash
# Terminal 2
cd Documents/GhostHandDesk/client

# Compiler et lancer Tauri
cargo tauri dev
```

**✅ Validation:**
- Compilation Rust (~1-2 min première fois)
- Serveur Vite démarre sur http://localhost:5173
- Fenêtre Tauri s'ouvre avec l'interface
- Header affiche: `Device ID: GHD-xxxxxxxxxx`
- Status: "Déconnecté"
- Dialog de connexion visible

**📝 Noter le Device ID:** `GHD-abc123def456` (exemple)

#### 5. Lancer Client Remote (Machine B)

```bash
# Terminal 3 (ou autre machine)
cd Documents/GhostHandDesk/client

# Lancer deuxième instance Tauri
cargo tauri dev
```

**✅ Validation:**
- Même comportement que Machine A
- Device ID différent visible

#### 6. Vérifier Enregistrement des Clients

```bash
# Terminal 4
curl -k https://localhost:8443/stats
```

**✅ Résultat Attendu:**
```json
{
  "total_clients": 2,
  "uptime": "2m15s",
  "max_clients": 1000
}
```

#### 7. Initier la Connexion

**Sur Machine B (Client):**
1. Dans le dialog de connexion
2. Entrer le Device ID de Machine A: `GHD-abc123def456`
3. Mot de passe: (laisser vide)
4. Cliquer "Se connecter"

**✅ Validation Machine B:**
- Status passe à "Connexion..."
- Logs console (F12) montrent:
  ```
  [INFO] Connexion au device GHD-abc123def456
  [INFO] Création de l'offre WebRTC
  [INFO] Offre envoyée au serveur
  ```

**✅ Validation Machine A (Host):**
- Notification de connexion entrante (future feature)
- Logs console montrent:
  ```
  [INFO] Offre reçue de GHD-xyz789
  [INFO] Création de la réponse WebRTC
  [INFO] Réponse envoyée
  ```

**✅ Validation Serveur (Terminal 1):**
```
[HUB] Client enregistré: GHD-abc123def456
[HUB] Client enregistré: GHD-xyz789
[SIGNALING] Offer: GHD-xyz789 -> GHD-abc123def456
[SIGNALING] Answer: GHD-abc123def456 -> GHD-xyz789
[SIGNALING] ICE Candidate: GHD-xyz789 -> GHD-abc123def456
[SIGNALING] ICE Candidate: GHD-abc123def456 -> GHD-xyz789
[WEBRTC] Connexion établie: GHD-xyz789 <-> GHD-abc123def456
```

#### 8. Vérifier le Streaming Vidéo

**Sur Machine B:**
- Canvas affiche l'écran de Machine A
- Indicateurs visibles:
  - **FPS:** 25-30 (idéalement)
  - **Latence:** < 50ms (LAN)
  - **Bitrate:** ~4000 kbps

**✅ Validation Visuelle:**
- L'image est fluide (pas de freeze)
- Les mouvements souris/fenêtre sur Machine A se reflètent immédiatement
- Aucun artefact vidéo majeur

#### 9. Tester le Contrôle Souris

**Sur Machine B:**
1. Déplacer la souris sur le canvas
2. Cliquer sur une icône/fenêtre visible
3. Faire défiler (scroll)

**✅ Validation Machine A:**
- Le curseur bouge en temps réel
- Les clics s'exécutent correctement
- Le scroll fonctionne

**🐛 Troubleshooting:**
- Si curseur décalé: Vérifier scaling du canvas (doit être proportionnel)
- Si clics non détectés: Vérifier data channel (F12 Console)

#### 10. Tester le Contrôle Clavier

**Sur Machine B:**
1. Focus sur le canvas (clic)
2. Ouvrir une application (ex: Notepad)
3. Taper du texte: "Test GhostHandDesk 123"

**✅ Validation Machine A:**
- Notepad s'ouvre
- Le texte apparaît correctement
- Les touches spéciales fonctionnent (Enter, Backspace, etc.)

#### 11. Test de Performance

**Mesures à relever (Machine B, pendant 60 secondes):**

| Métrique | Valeur Attendue | Valeur Réelle |
|----------|-----------------|---------------|
| FPS moyen | ≥ 25 | ___ |
| FPS min | ≥ 15 | ___ |
| Latence moyenne | < 50ms | ___ |
| Latence max | < 100ms | ___ |
| CPU Client (%) | < 20% | ___ |
| CPU Host (%) | < 30% | ___ |
| RAM Client (MB) | < 200 | ___ |
| RAM Host (MB) | < 300 | ___ |
| Bande passante (Mbps) | 3-5 | ___ |

**Commandes de mesure:**
```powershell
# CPU et RAM (Windows)
Get-Process GhostHandDesk | Select-Object CPU,WorkingSet

# Bande passante (Windows Resource Monitor)
resmon.exe
```

#### 12. Test de Stabilité

**Scénario:**
1. Maintenir la connexion pendant 5 minutes
2. Alterner entre fenêtres/applications sur Machine A
3. Vérifier que le streaming reste stable

**✅ Validation:**
- Aucune déconnexion
- FPS reste stable (variation < 10%)
- Pas de memory leak (RAM stable)

#### 13. Test de Déconnexion

**Sur Machine B:**
1. Cliquer sur "Déconnecter"

**✅ Validation:**
- Status passe à "Déconnecté"
- Canvas se vide
- Dialog de connexion réapparaît
- Logs serveur montrent:
  ```
  [HUB] Client déconnecté: GHD-xyz789
  ```

**✅ Validation Ressources:**
- WebRTC connection fermée proprement
- Data channels libérés
- Pas de processus zombie

---

## 🧪 Scénario 2 : Test Multi-Résolution

### Objectif
Valider que le streaming s'adapte aux différentes résolutions d'écran.

### Configurations à Tester

| Configuration | Résolution Host | Résolution Client | FPS Attendu | Bitrate |
|---------------|-----------------|-------------------|-------------|---------|
| Basse qualité | 1920x1080 | Tout | 15 | 2000 kbps |
| Moyenne qualité | 1920x1080 | Tout | 30 | 4000 kbps |
| Haute qualité | 2560x1440 | Tout | 30 | 6000 kbps |
| 4K (si supporté) | 3840x2160 | Tout | 15 | 8000 kbps |

### Procédure

Pour chaque configuration:
1. Ajuster paramètres dans SettingsPanel (⚙️)
2. Reconnecter
3. Mesurer FPS/latence réelles
4. Noter qualité visuelle subjective (1-5)

**✅ Résultats Attendus:**
- Toutes les résolutions fonctionnent
- Scaling automatique correct
- Pas de crash

---

## 🧪 Scénario 3 : Test de Robustesse

### Test 3.1 : Déconnexion Réseau

**Procédure:**
1. Établir connexion normale
2. Désactiver Wi-Fi/Ethernet sur Machine B pendant 10s
3. Réactiver

**✅ Validation:**
- Status passe à "Déconnecté" ou "Reconnexion..."
- Après retour réseau, reconnexion automatique (future feature)
- Ou message clair invitant à reconnecter manuellement

### Test 3.2 : Crash du Serveur

**Procédure:**
1. Établir connexion normale
2. Arrêter le serveur Go (Ctrl+C dans Terminal 1)
3. Observer comportement clients

**✅ Validation:**
- Les clients détectent la perte de signaling
- Message d'erreur clair
- Pas de crash client
- Après redémarrage serveur, possibilité de reconnecter

### Test 3.3 : Connexions Multiples

**Procédure:**
1. Lancer 1 serveur
2. Lancer 3 clients (Host + 2 Remote)
3. Client Remote 1 se connecte au Host
4. Client Remote 2 tente de se connecter au Host

**✅ Validation:**
- Le serveur gère les 3 clients
- Les 2 connexions P2P fonctionnent simultanément (future feature multi-peer)
- Ou gestion propre de "Host déjà occupé"

### Test 3.4 : Timeout de Connexion

**Procédure:**
1. Lancer client sans serveur
2. Tenter de se connecter

**✅ Validation:**
- Timeout après 10-15 secondes
- Message d'erreur clair: "Serveur de signalement inaccessible"
- Pas de freeze UI

---

## 🧪 Scénario 4 : Test de Sécurité

### Test 4.1 : Connexion avec Mot de Passe

**Procédure:**
1. Configurer mot de passe dans Settings (Machine A)
2. Machine B se connecte avec mauvais mot de passe
3. Machine B se connecte avec bon mot de passe

**✅ Validation:**
- Échec avec mauvais mot de passe
- Succès avec bon mot de passe
- Mot de passe haché (pas en clair dans logs)

### Test 4.2 : Validation Device ID

**Procédure:**
1. Entrer Device ID invalide: `INVALID-123`
2. Entrer Device ID inexistant: `GHD-999999999999`
3. Entrer Device ID valide

**✅ Validation:**
- Rejet Device ID malformé
- Message "Device non trouvé" pour ID inexistant
- Succès avec ID valide

### Test 4.3 : Certificats TLS

**Procédure:**
```bash
# Vérifier certificat serveur
openssl s_client -connect localhost:8443 -showcerts
```

**✅ Validation:**
- Certificat valide
- Cipher suite sécurisé (TLS 1.2+)
- Pas d'avertissement majeur

---

## 🧪 Scénario 5 : Test de Compatibilité Codec

### Test 5.1 : H.264 (FFmpeg)

**Prérequis:** FFmpeg installé

**Procédure:**
```bash
cd client
cargo build --release --features ffmpeg
cargo tauri dev
```

**✅ Validation:**
- Logs montrent: "Encodeur FFmpeg initialisé"
- Compression ~100x par rapport à RGBA brut
- FPS > 25

### Test 5.2 : JPEG Fallback

**Prérequis:** FFmpeg NON installé

**Procédure:**
```bash
cd client
cargo build --release --no-default-features
cargo tauri dev
```

**✅ Validation:**
- Logs montrent: "FFmpeg non disponible, fallback JPEG"
- Streaming fonctionne quand même
- FPS réduit (~15-20) à cause de compression moins efficace

---

## 📊 Rapport de Tests

### Template de Rapport

Copier ce template dans `E2E_TEST_RESULTS.md`:

```markdown
# Résultats Tests End-to-End - GhostHandDesk

**Date:** ____________________
**Testeur:** ____________________
**Version:** 0.1.0

## Configuration Système

**Machine Host:**
- OS: ____________________
- CPU: ____________________
- RAM: ____________________
- Résolution: ____________________

**Machine Remote:**
- OS: ____________________
- CPU: ____________________
- RAM: ____________________
- Résolution: ____________________

**Réseau:**
- Type: LAN / WAN / VPN
- Latence ping: ____ ms
- Bande passante: ____ Mbps

## Résultats Scénarios

### ✅ Scénario 1 : Connexion Locale
- [ ] Serveur démarre correctement
- [ ] Clients s'enregistrent
- [ ] Connexion WebRTC établie
- [ ] Streaming vidéo fonctionnel
- [ ] Contrôle souris fonctionnel
- [ ] Contrôle clavier fonctionnel
- [ ] Performance acceptable (FPS: ____, Latence: ____)

**Notes:** ____________________

### ✅ Scénario 2 : Multi-Résolution
- [ ] 1920x1080 @ 30 FPS
- [ ] 2560x1440 @ 30 FPS
- [ ] Scaling correct

**Notes:** ____________________

### ✅ Scénario 3 : Robustesse
- [ ] Déconnexion réseau gérée
- [ ] Crash serveur géré
- [ ] Connexions multiples gérées
- [ ] Timeout fonctionnel

**Notes:** ____________________

### ✅ Scénario 4 : Sécurité
- [ ] Mot de passe fonctionne
- [ ] Validation Device ID
- [ ] TLS actif

**Notes:** ____________________

### ✅ Scénario 5 : Codec
- [ ] H.264 (FFmpeg) fonctionne
- [ ] JPEG fallback fonctionne

**Notes:** ____________________

## Bugs Identifiés

| # | Sévérité | Description | Reproduction |
|---|----------|-------------|--------------|
| 1 |          |             |              |

## Recommandations

1. ____________________
2. ____________________
3. ____________________

## Conclusion

**Status Global:** ✅ PASS / ⚠️ PASS avec warnings / ❌ FAIL

**Prêt pour production:** OUI / NON

**Signature:** ____________________
```

---

## 🚀 Automatisation des Tests

### Script PowerShell

Créer `scripts/run-e2e-tests.ps1`:

```powershell
# Script d'automatisation tests E2E
param(
    [switch]$FullSuite,
    [switch]$QuickTest,
    [switch]$ServerOnly
)

Write-Host "🧪 GhostHandDesk - Tests End-to-End" -ForegroundColor Cyan
Write-Host "====================================`n"

# Vérification prérequis
Write-Host "Vérification prérequis..." -ForegroundColor Yellow

$goInstalled = Get-Command go -ErrorAction SilentlyContinue
$ffmpegInstalled = Get-Command ffmpeg -ErrorAction SilentlyContinue

if (-not $goInstalled) {
    Write-Host "❌ Go non installé" -ForegroundColor Red
    exit 1
}

if (-not $ffmpegInstalled) {
    Write-Host "⚠️  FFmpeg non installé (fallback JPEG sera utilisé)" -ForegroundColor Yellow
}

Write-Host "✅ Prérequis OK`n" -ForegroundColor Green

# Générer certificats
if (-not (Test-Path "server/certs/server.crt")) {
    Write-Host "Génération certificats TLS..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Force -Path "server/certs" | Out-Null

    & openssl req -x509 -newkey rsa:4096 -nodes `
        -keyout server/certs/server.key `
        -out server/certs/server.crt `
        -days 365 `
        -subj "/CN=localhost"

    Write-Host "✅ Certificats générés`n" -ForegroundColor Green
}

# Lancer serveur
Write-Host "Démarrage serveur de signalement..." -ForegroundColor Yellow

$serverJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cd server
    go run cmd/signaling/main.go
}

Start-Sleep -Seconds 3

# Test santé serveur
try {
    $health = Invoke-RestMethod -Uri "https://localhost:8443/health" -SkipCertificateCheck
    if ($health.status -eq "healthy") {
        Write-Host "✅ Serveur démarré et fonctionnel`n" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Serveur non accessible" -ForegroundColor Red
    Stop-Job $serverJob
    exit 1
}

if ($ServerOnly) {
    Write-Host "Mode ServerOnly - Serveur en cours d'exécution"
    Write-Host "Appuyez sur Ctrl+C pour arrêter"
    Wait-Job $serverJob
    exit 0
}

# Compiler client
Write-Host "Compilation du client..." -ForegroundColor Yellow
cd client
cargo build --release
Write-Host "✅ Client compilé`n" -ForegroundColor Green

# Tests unitaires
Write-Host "Exécution tests unitaires..." -ForegroundColor Yellow
cargo test --lib
Write-Host "✅ Tests unitaires OK`n" -ForegroundColor Green

# Tests d'intégration
Write-Host "Exécution tests d'intégration..." -ForegroundColor Yellow
cargo test --test integration_test
Write-Host "✅ Tests d'intégration OK`n" -ForegroundColor Green

# Nettoyage
Write-Host "`nArrêt du serveur..." -ForegroundColor Yellow
Stop-Job $serverJob
Remove-Job $serverJob

Write-Host "✅ Tests terminés avec succès !" -ForegroundColor Green
```

**Utilisation:**
```powershell
# Tests rapides (unitaires + intégration)
.\scripts\run-e2e-tests.ps1 -QuickTest

# Suite complète
.\scripts\run-e2e-tests.ps1 -FullSuite

# Serveur uniquement (pour tests manuels)
.\scripts\run-e2e-tests.ps1 -ServerOnly
```

---

## 📝 Checklist Finale

Avant de déclarer GhostHandDesk "Production Ready":

### Fonctionnel
- [ ] Serveur Go démarre sans erreur
- [ ] Clients Tauri se compilent et lancent
- [ ] Connexion WebRTC P2P fonctionne
- [ ] Streaming vidéo H.264 fonctionne
- [ ] Contrôle souris temps réel (< 50ms)
- [ ] Contrôle clavier fonctionnel
- [ ] Déconnexion propre

### Performance
- [ ] FPS ≥ 25 en LAN
- [ ] Latence < 50ms en LAN
- [ ] CPU < 30% (Host et Client)
- [ ] RAM < 300MB (Host et Client)
- [ ] Pas de memory leak (5 min stable)

### Robustesse
- [ ] Déconnexion réseau gérée
- [ ] Timeout de connexion
- [ ] Validation des inputs
- [ ] Gestion d'erreurs propre

### Sécurité
- [ ] TLS actif
- [ ] Certificats valides
- [ ] Mot de passe haché
- [ ] Pas de secrets en clair dans logs

### Documentation
- [ ] README complet
- [ ] QUICKSTART disponible
- [ ] Ce guide E2E
- [ ] Commentaires code en français

---

## 🆘 Support

**En cas de problème:**

1. Consulter `SESSION_REPORT.md` pour l'état du projet
2. Vérifier logs:
   - Serveur: stdout Terminal 1
   - Client backend: stdout Terminal 2
   - Client frontend: F12 Console
3. Relancer tests unitaires: `cargo test`
4. Consulter issues GitHub (à créer)

**Logs de debug:**
```bash
# Serveur Go avec debug
LOG_LEVEL=debug go run cmd/signaling/main.go

# Client Rust avec traces
RUST_LOG=debug cargo tauri dev
```

---

**🎉 Bon testing ! Le projet est prêt à être validé.**
