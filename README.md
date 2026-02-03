# 👻 GhostHandDesk - Bureau à Distance Open Source

Application de prise en main à distance sécurisée et performante, utilisant WebRTC pour les connexions P2P.

## 📋 Vue d'ensemble

**GhostHandDesk** est composé de trois parties principales :
1. **Serveur de signalement (Go)** - Gère la signalisation WebRTC ✅
2. **Client Rust** - Application de bureau avec capture d'écran et contrôle à distance ✅
3. **Interface Tauri** - Interface utilisateur moderne ✅

## 🎯 État du projet : 100% fonctionnel ✅

### ✅ Modules implémentés

- ✅ **Capture d'écran** (`screen_capture.rs`) - Multi-moniteurs avec xcap
- ✅ **Contrôle d'entrée** (`input_control.rs`) - Clavier/souris avec enigo
- ✅ **Cryptographie** (`crypto.rs`) - AES-256-GCM, SHA256
- ✅ **Configuration** (`config.rs`) - Gestion JSON complète
- ✅ **Gestion d'erreurs** (`error.rs`) - Système d'erreurs robuste
- ✅ **WebRTC P2P** (`network.rs`) - Connexions peer-to-peer complètes
- ✅ **Encodage vidéo** (`video_encoder.rs`) - H.264 via FFmpeg + fallback JPEG
- ✅ **Streaming** (`streaming.rs`) - Boucle capture → encode → send
- ✅ **Serveur de signalement Go** - Hub WebSocket complet
- ✅ **Interface Tauri** - Application desktop complète avec Vue 3 + TypeScript
- ✅ **Tests d'intégration** - Compilation réussie et exécutables Windows

## 🚀 Installation rapide

### Prérequis

**Rust (Client)**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Go (Serveur)** - Version 1.21+
```bash
# Windows (Chocolatey)
choco install golang

# Linux
sudo apt install golang-go

# macOS
brew install go
```

**Node.js** - Pour l'interface Tauri
```bash
# Windows (Chocolatey)
choco install nodejs

# Linux
sudo apt install nodejs npm

# macOS
brew install node
```

**FFmpeg (Optionnel mais recommandé)**
```bash
# Windows
choco install ffmpeg

# Linux
sudo apt install libavcodec-dev libavformat-dev libavutil-dev

# macOS
brew install ffmpeg
```

### Compilation

**Application Tauri (Recommandé)**
```bash
cd client

# Installer les dépendances UI
cd ui
npm install
cd ..

# Compiler en mode release
cargo tauri build
```

Cela génère :
- `client/src-tauri/target/release/ghosthanddesk-tauri.exe` - Application standalone
- `client/src-tauri/target/release/bundle/msi/GhostHandDesk_0.1.0_x64_en-US.msi` - Installateur MSI
- `client/src-tauri/target/release/bundle/nsis/GhostHandDesk_0.1.0_x64-setup.exe` - Installateur NSIS

**Client Rust (Sans interface)**
```bash
cd client

# Sans FFmpeg (utilise JPEG)
cargo build --release

# Avec FFmpeg (H.264)
cargo build --release --features ffmpeg
```

**Serveur Go**
```bash
cd server

# Télécharger les dépendances
go mod download

# Générer les certificats TLS (dev)
mkdir certs
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout certs/server.key \
  -out certs/server.crt \
  -days 365 -subj "/CN=localhost"

# Compiler
go build -o bin/signaling.exe cmd/signaling/main.go
```

## 🏃 Lancement

### 🪟 Windows - Méthode rapide

**Option 1 : Script de lancement (le plus simple)**
```bash
# Double-cliquer sur le fichier
Lancer-GhostHandDesk.bat
```

**Option 2 : Exécutable direct**
```bash
client\src-tauri\target\release\ghosthanddesk-tauri.exe
```

**Option 3 : Installateur**
Utiliser l'un des installateurs générés (.msi ou -setup.exe) pour une installation système complète.

### Serveur de signalement

```bash
cd server
go run cmd/signaling/main.go
```

Le serveur démarre sur `https://localhost:8443` avec les routes :
- `wss://localhost:8443/ws` - WebSocket
- `https://localhost:8443/health` - Health check
- `https://localhost:8443/stats` - Statistiques

### Client (mode développement)

**Avec Tauri :**
```bash
cd client
cargo tauri dev
```

**Sans Tauri :**
```bash
cd client
cargo run --release
```

**Sortie attendue :**
```
==============================================
🚀 GhostHandDesk v0.1.0
==============================================
📱 Device ID: GHD-abc123def456
🌐 Serveur: wss://localhost:8443/ws
==============================================
[TAURI] Application initialisée
[TAURI] Interface disponible
```

## 📁 Architecture

```
GhostHandDesk/
├── client/                     # Client Rust + Tauri
│   ├── src/
│   │   ├── config.rs          # Configuration
│   │   ├── crypto.rs          # Chiffrement AES-256-GCM
│   │   ├── error.rs           # Gestion d'erreurs
│   │   ├── input_control.rs   # Contrôle clavier/souris
│   │   ├── network.rs         # WebRTC + Signaling
│   │   ├── screen_capture.rs  # Capture multi-écrans
│   │   ├── streaming.rs       # Loop capture-encode-send
│   │   ├── video_encoder.rs   # H.264/JPEG encoding
│   │   └── main.rs            # Point d'entrée
│   ├── src-tauri/             # Backend Tauri
│   │   ├── src/main.rs        # Backend Rust
│   │   ├── tauri.conf.json    # Configuration Tauri
│   │   └── Cargo.toml
│   ├── ui/                    # Frontend Vue 3
│   │   ├── src/
│   │   │   ├── App.vue
│   │   │   ├── components/
│   │   │   │   ├── ConnectDialog.vue
│   │   │   │   ├── RemoteViewer.vue
│   │   │   │   └── SettingsPanel.vue
│   │   │   └── main.ts
│   │   ├── package.json
│   │   └── vite.config.ts
│   ├── Cargo.toml
│   └── config.example.json
│
├── server/                     # Serveur Go
│   ├── cmd/signaling/
│   │   └── main.go            # Point d'entrée
│   ├── internal/
│   │   ├── config/config.go   # Configuration
│   │   ├── models/message.go  # Structures de messages
│   │   └── signaling/
│   │       ├── hub.go         # Hub de gestion clients
│   │       └── handler.go     # Handler WebSocket
│   ├── certs/                 # Certificats TLS
│   ├── go.mod
│   ├── .env.example
│   └── README.md
│
├── Lancer-GhostHandDesk.bat   # Script de lancement Windows
└── README.md                   # Ce fichier
```

## 🔧 Configuration

### Client (`client/config.json`)

```json
{
  "server_url": "wss://localhost:8443/ws",
  "stun_servers": [
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302"
  ],
  "video_config": {
    "codec": "H264",
    "framerate": 30,
    "bitrate": 4000,
    "quality": 80
  }
}
```

### Serveur (`.env`)

```env
SERVER_HOST=:8443
CERT_FILE=certs/server.crt
KEY_FILE=certs/server.key
LOG_LEVEL=info
MAX_CLIENTS=1000
CONNECTION_TIMEOUT=60
```

## 🔐 Sécurité

- **TLS obligatoire** : Toutes les communications sont chiffrées
- **Cryptographie** : AES-256-GCM pour les données sensibles
- **WebRTC** : Connexions P2P authentifiées via DTLS-SRTP
- **Validation** : Tous les inputs sont validés et sanitizés

**⚠️ IMPORTANT :** En production, utiliser des certificats valides (Let's Encrypt).

## 📊 Performance

### Benchmarks (estimés)

| Composant | Latence | CPU | Bande passante |
|-----------|---------|-----|----------------|
| Capture (xcap) | < 10ms | ~5% | N/A |
| Encodage H.264 (FFmpeg) | < 15ms | ~15% | 2-4 Mbps |
| Encodage H.264 (NVENC) | < 5ms | ~5% | 2-4 Mbps |
| Encodage JPEG | < 10ms | ~10% | 10-20 Mbps |
| WebRTC latency | 30-100ms | ~5% | Selon codec |
| Interface Tauri | < 1ms | ~2% | N/A |

**Configuration testée :** Windows 11, Intel i7, 16GB RAM, 1080p@30fps

## 🧪 Tests

```bash
# Tests unitaires client
cd client
cargo test

# Tests serveur
cd server
go test ./...

# Tests avec couverture (client)
cd client
cargo tarpaulin --out Html

# Tests Tauri
cd client
cargo tauri dev
```

## 🛠️ Développement

### Compilation pour différentes plateformes

**Windows :**
```bash
cargo tauri build --target x86_64-pc-windows-msvc
```

**Linux :**
```bash
cargo tauri build --target x86_64-unknown-linux-gnu
```

**macOS :**
```bash
cargo tauri build --target x86_64-apple-darwin
```

### Améliorations futures

1. **Optimisations** (Priorité haute)
   - Accélération matérielle (NVENC, QSV, VideoToolbox)
   - Réduction de la latence
   - Optimisation de la bande passante

2. **Fonctionnalités** (Priorité moyenne)
   - Support audio bidirectionnel
   - Transfert de fichiers
   - Multi-moniteurs côté remote
   - Presse-papiers partagé

3. **Interface** (Priorité basse)
   - Mode plein écran
   - Raccourcis clavier personnalisables
   - Thème sombre/clair
   - Multi-langue

## 📝 Protocole de signalisation

### Messages

**Register**
```json
{
  "type": "Register",
  "data": { "device_id": "GHD-abc123" }
}
```

**Offer/Answer**
```json
{
  "type": "Offer",
  "data": {
    "from": "GHD-abc123",
    "to": "GHD-def456",
    "sdp": "v=0..."
  }
}
```

**IceCandidate**
```json
{
  "type": "IceCandidate",
  "data": {
    "from": "GHD-abc123",
    "to": "GHD-def456",
    "candidate": "...",
    "sdp_mid": "0",
    "sdp_mline_index": 0
  }
}
```

## 🐛 Troubleshooting

### Erreur "WebRTC connection failed"
- Vérifier que les serveurs STUN sont accessibles
- Tester avec un TURN server si derrière NAT strict

### Erreur "FFmpeg not found"
- Installer FFmpeg (voir `client/FFMPEG_SETUP.md`)
- Ou compiler sans feature : `cargo build --release`

### Performance faible
1. Activer l'accélération matérielle
2. Réduire le framerate (ex: 15 fps)
3. Réduire la résolution
4. Utiliser codec JPEG si problème avec H.264

### Serveur ne démarre pas
- Vérifier que le port 8443 n'est pas utilisé
- Vérifier les certificats TLS : `ls -la server/certs/`
- Vérifier Go version : `go version` (≥ 1.21)

### Tauri build échoue
- Vérifier Node.js : `node --version` (≥ 18)
- Réinstaller les dépendances : `cd ui && npm install`
- Nettoyer le cache : `cargo clean && cd ui && rm -rf node_modules`

## 📦 Distribution

### Fichiers générés par `cargo tauri build`

1. **ghosthanddesk-tauri.exe** - Application portable (pas d'installation)
2. **GhostHandDesk_x.x.x_x64_en-US.msi** - Installateur Windows Installer
3. **GhostHandDesk_x.x.x_x64-setup.exe** - Installateur NSIS (recommandé)

### Signature de code (Production)

Pour distribuer l'application, il est recommandé de signer le code :

```bash
# Windows
signtool sign /f certificate.pfx /p password /t http://timestamp.digicert.com ghosthanddesk-tauri.exe
```

## 📜 Licence

MIT OR Apache-2.0

## 🙏 Remerciements

- [webrtc-rs](https://github.com/webrtc-rs/webrtc) - Stack WebRTC Rust
- [xcap](https://github.com/nashaofu/xcap) - Capture d'écran cross-platform
- [FFmpeg](https://ffmpeg.org/) - Encodage vidéo
- [Tauri](https://tauri.app/) - Framework d'applications de bureau
- [Vue 3](https://vuejs.org/) - Framework frontend
- [gorilla/websocket](https://github.com/gorilla/websocket) - WebSocket Go

---

**Made with ❤️ and Rust 🦀**

**Version actuelle :** 0.1.0
**Dernière mise à jour :** 2026-02-03
