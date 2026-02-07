# 🚀 Guide de Démarrage Rapide - GhostHandDesk

Ce guide vous permet de lancer GhostHandDesk en moins de 10 minutes.

## ⚡ Installation express (Windows)

### Étape 1 : Installer les dépendances (5 min)

**Ouvrir PowerShell en Administrateur :**

```powershell
# Installer Chocolatey (si pas déjà installé)
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# Installer Go
choco install golang -y

# Installer FFmpeg
choco install ffmpeg -y

# Redémarrer le terminal pour charger les nouvelles variables PATH
```

**Vérifier les installations :**
```bash
go version       # Devrait afficher "go version go1.21.x"
ffmpeg -version  # Devrait afficher la version FFmpeg
cargo --version  # Devrait afficher la version Rust
```

### Étape 2 : Compiler le serveur (2 min)

```bash
cd Documents/GhostHandDesk/server

# Télécharger les dépendances Go
go mod download

# Générer les certificats TLS auto-signés (développement uniquement)
mkdir certs
openssl req -x509 -newkey rsa:4096 -nodes -keyout certs/server.key -out certs/server.crt -days 365 -subj "/CN=localhost"

# Compiler le serveur
go build -o bin/signaling.exe cmd/signaling/main.go
```

### Étape 3 : Compiler le client (3 min)

```bash
cd ../client

# Compilation avec encodage H.264
cargo build --release --features ffmpeg

# OU compilation sans FFmpeg (fallback JPEG)
cargo build --release
```

## 🎬 Lancement

### Terminal 1 : Serveur de signalement

```bash
cd server
go run cmd/signaling/main.go
```

**Sortie attendue :**
```
[MAIN] Configuration chargée: Host=:8443, CertFile=certs/server.crt, MaxClients=1000
[MAIN] Hub de signalement démarré
[MAIN] Serveur de signalement démarré sur :8443
[MAIN] Routes disponibles:
  - wss://localhost:8443/ws (WebSocket)
  - https://localhost:8443/health (Health check)
  - https://localhost:8443/stats (Statistiques)
```

### Terminal 2 : Client (Device 1)

```bash
cd client
cargo run --release
```

**Sortie attendue :**
```
GhostHandDesk Client v0.1.0
Starting remote desktop client...
Device ID: GHD-abc123def456
Available displays:
  - Display 0: \\.\DISPLAY1 (1920x1080) at (0, 0) [PRIMARY]
Captured frame: 1920x1080, 8294400 bytes
Encoded frame: 45234 bytes (183x reduction)
Status: Ready (not connected)
```

**Notez le Device ID !** (ex: `GHD-abc123def456`)

### Terminal 3 : Client (Device 2) - Optionnel

```bash
cd client
cargo run --release
```

Notez également son Device ID.

## 🧪 Tests de base

### Test 1 : Health check serveur

```bash
curl -k https://localhost:8443/health
```

**Attendu :**
```json
{
  "status": "healthy",
  "clients": 2
}
```

### Test 2 : Statistiques

```bash
curl -k https://localhost:8443/stats
```

**Attendu :**
```json
{
  "total_clients": 2,
  "uptime": "2m30s",
  "max_clients": 1000
}
```

### Test 3 : Capture d'écran

Le client devrait afficher :
```
Captured frame: 1920x1080, 8294400 bytes
```

### Test 4 : Encodage vidéo

**Avec FFmpeg (H.264) :**
```
Encoded frame: 45234 bytes (183x reduction)
```

**Sans FFmpeg (JPEG) :**
```
Encoded frame: 152341 bytes (54x reduction)
```

## 🔌 Connexion entre deux clients (manuel)

**Note :** L'interface Tauri n'étant pas encore implémentée, la connexion manuelle nécessite des modifications de code temporaires.

### Option 1 : Mode test (dans le code)

Éditez `client/src/main.rs` et ajoutez avant la ligne finale :

```rust
// Tester la connexion au serveur
info!("Connexion au serveur de signalement...");
let mut session = network::SessionManager::new(config.clone(), device_id.clone());
session.initialize().await?;
info!("Connecté au serveur !");

// Pour se connecter à un autre device :
// session.connect_to_device("GHD-TARGET-ID".to_string(), None).await?;
```

### Option 2 : Attendre l'interface Tauri

L'interface permettra de :
1. Voir son propre Device ID
2. Entrer le Device ID de la machine cible
3. Établir la connexion WebRTC automatiquement
4. Afficher le streaming vidéo

## 🐛 Problèmes courants

### Erreur : "go: command not found"

**Solution :** Redémarrer le terminal après installation de Go.

### Erreur : "ffmpeg-sys-next build failed"

**Solution :**
```bash
# Vérifier FFmpeg
ffmpeg -version

# Si absent, installer
choco install ffmpeg

# Recompiler sans FFmpeg
cargo build --release  # Sans --features ffmpeg
```

### Erreur : "Address already in use (port 8443)"

**Solution :**
```bash
# Trouver le processus
netstat -ano | findstr :8443

# Tuer le processus (remplacer PID)
taskkill /PID <PID> /F

# Ou changer le port dans server/.env
SERVER_HOST=:9443
```

### Erreur : "certificate signed by unknown authority"

**Solution :** Normal en développement avec certificats auto-signés. Utiliser `-k` avec curl :
```bash
curl -k https://localhost:8443/health
```

### Performance faible

**Solutions :**
1. Réduire le framerate :
   ```json
   // client/config.json
   "framerate": 15  // Au lieu de 30
   ```

2. Réduire le bitrate :
   ```json
   "bitrate": 2000  // Au lieu de 4000
   ```

3. Activer l'accélération matérielle (voir `client/FFMPEG_SETUP.md`)

## 📊 Vérifier que tout fonctionne

### Checklist

- [ ] Go installé et dans PATH : `go version`
- [ ] FFmpeg installé (optionnel) : `ffmpeg -version`
- [ ] Rust installé : `cargo --version`
- [ ] Serveur compile : `cd server && go build cmd/signaling/main.go`
- [ ] Client compile : `cd client && cargo build --release`
- [ ] Serveur démarre : Logs visibles sur port 8443
- [ ] Client démarre : Device ID affiché
- [ ] Health check OK : `curl -k https://localhost:8443/health`
- [ ] Capture fonctionne : "Captured frame" dans les logs
- [ ] Encodage fonctionne : "Encoded frame" dans les logs

### Tout est vert ? 🎉

**Félicitations !** Votre environnement GhostHandDesk est opérationnel.

**Prochaine étape :** Implémenter l'interface Tauri (Tâche #5)

## 📚 Ressources

- **Documentation complète :** `README.md`
- **Guide FFmpeg :** `client/FFMPEG_SETUP.md`
- **Serveur Go :** `server/README.md`
- **Rapport d'implémentation :** `IMPLEMENTATION_REPORT.md`

## 🆘 Support

**Problèmes :** Ouvrir une issue sur GitHub
**Questions :** Consulter `IMPLEMENTATION_REPORT.md` pour les détails techniques

---

**Temps total estimé :** 10 minutes (avec dépendances déjà installées)
