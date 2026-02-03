# 👻 GhostHandDesk - Bureau à Distance Open Source

Application de prise en main à distance sécurisée et performante, utilisant WebRTC pour les connexions P2P.

## 📋 Vue d'ensemble

**GhostHandDesk** est composé de trois parties principales :
1. **Serveur de signalement (Go)** - Gère la signalisation WebRTC ✅
2. **Client Rust** - Application de bureau avec capture d'écran et contrôle à distance ✅
3. **Interface Tauri** - Interface utilisateur moderne ⏳

## 🎯 État du projet : 90% fonctionnel

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

### ⏳ En cours

- ⏳ **Interface Tauri** (0%) - À implémenter
- ⏳ **Tests d'intégration** (0%) - À créer

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

**Client Rust**
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

### Serveur de signalement

```bash
cd server
go run cmd/signaling/main.go
```

Le serveur démarre sur `https://localhost:8443` avec les routes :
- `wss://localhost:8443/ws` - WebSocket
- `https://localhost:8443/health` - Health check
- `https://localhost:8443/stats` - Statistiques

### Client

```bash
cd client
cargo run --release
```

**Sortie attendue :**
```
GhostHandDesk Client v0.1.0
Device ID: GHD-abc123def456
Status: Ready
```

## 📁 Architecture

```
GhostHandDesk/
├── client/                     # Client Rust
│   ├── src/
│   │   ├── config.rs          # Configuration
│   │   ├── crypto.rs          # Chiffrement AES-256-GCM
│   │   ├── error.rs           # Gestion d'erreurs
│   │   ├── input_control.rs   # Contrôle clavier/souris
│   │   ├── network.rs         # WebRTC + Signaling
│   │   ├── screen_capture.rs  # Capture multi-écrans
│   │   ├── streaming.rs       # Loop capture-encode-send
│   │   ├── video_encoder.rs   # H.264/JPEG encoding
│   │   ├── ui/mod.rs          # Interface (à implémenter)
│   │   └── main.rs            # Point d'entrée
│   ├── Cargo.toml
│   ├── FFMPEG_SETUP.md        # Guide FFmpeg
│   └── config.example.json    # Config exemple
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
```

## 🛠️ Développement

### Prochaines étapes

1. **Interface Tauri** (Priorité haute)
   - Installation : `cargo install tauri-cli`
   - Frontend Vue 3 + TypeScript
   - Composants : ConnectDialog, RemoteViewer, Settings

2. **Tests d'intégration** (Priorité moyenne)
   - Scénarios end-to-end
   - Tests de performance
   - Tests de robustesse

3. **Améliorations** (Priorité basse)
   - Accélération matérielle (NVENC, QSV)
   - Support audio
   - Transfert de fichiers
   - Multi-moniteurs côté remote

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

## 📜 Licence

MIT OR Apache-2.0

## 🙏 Remerciements

- [webrtc-rs](https://github.com/webrtc-rs/webrtc) - Stack WebRTC Rust
- [xcap](https://github.com/nashaofu/xcap) - Capture d'écran cross-platform
- [FFmpeg](https://ffmpeg.org/) - Encodage vidéo
- [Tauri](https://tauri.app/) - Framework d'applications de bureau
- [gorilla/websocket](https://github.com/gorilla/websocket) - WebSocket Go

---

**Made with ❤️ and Rust 🦀**
