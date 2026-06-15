# GhostHandDesk
## Démonstration

<video controls width="100%" preload="none">
  <source src="https://mydepot.heiphaistos.org/Heiphaistos/GhostHandDesk/releases/download/v1.0.0/demo.mp4" type="video/mp4">\n</video>\n
[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/heiphaistos44-crypto/GhostHandDesk)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Go](https://img.shields.io/badge/go-1.20+-00ADD8.svg)](https://golang.org/)
[![Tests](https://img.shields.io/badge/tests-70%20passed-brightgreen.svg)]()

**GhostHandDesk** est une solution de bureau a distance open-source, securisee et performante, construite avec Rust, Go et WebRTC.

Un seul executable portable embarque le client (Tauri + Vue.js) et le serveur de signalement (Go), sans installation requise.

---

## Demarrage Rapide

### Option 1 : Executable Portable (recommande)

```bash
# Construire le package portable
BUILD-PORTABLE.bat

# Lancer l'application
GhostHandDesk-Portable/LANCER-GHOSTHANDDESK.bat
```

L'application demarre automatiquement le serveur de signalement embarque et ouvre l'interface.

### Option 2 : Depuis les sources

```bash
# Prerequis : Rust 1.70+, Node.js 18+, Go 1.20+

# Build complet (serveur Go + frontend Vue + client Tauri)
cd server && go build -o signaling-server.exe ./cmd/signaling/
cd ../client && npx tauri build
```

---

## Architecture

```
GhostHandDesk/
├── client/                     # Client (Rust + Tauri v2 + Vue.js 3)
│   ├── src/                    # Bibliotheque Rust (reseau, video, crypto, etc.)
│   ├── src-tauri/              # Application Tauri (commandes, etat, IPC)
│   └── ui/                     # Interface Vue.js 3 (TypeScript + Vite)
├── server/                     # Serveur de signalement (Go + WebSocket)
│   ├── cmd/signaling/          # Point d'entree
│   └── internal/signaling/     # Hub, handlers, rate limiting
├── docs/                       # Documentation technique
├── GhostHandDesk-Portable/     # Package portable (genere par le build)
├── BUILD-PORTABLE.bat          # Script de build
├── LANCER-APPLICATION.bat      # Lancement dev
└── VERIFIER-PREREQUIS.bat      # Verification des outils
```

### Stack Technique

| Composant | Technologie | Role |
|-----------|-------------|------|
| Client lib | Rust | Reseau, WebRTC, capture ecran, encodage video, crypto |
| Desktop app | Tauri v2 | Fenetre native, IPC, serveur embarque |
| Frontend | Vue.js 3 + TypeScript | Interface utilisateur |
| Signaling | Go + gorilla/websocket | Decouverte de pairs, relay WebSocket |
| Transport | WebRTC (DTLS-SRTP) | Connexion P2P, chiffrement natif |

---

## Fonctionnalites

### Connexion
- **Serveur embarque** : le serveur Go est integre dans l'executable via `include_bytes!`
- **Multi-instance** : detection automatique de serveur existant, fallback ports 9000-9004
- **Device ID unique** : genere avec timestamp + composante aleatoire
- **Demande de connexion** : popup d'acceptation/rejet sur l'instance cible
- **WebRTC P2P** : connexion directe entre pairs apres echange SDP/ICE

### Video
- **Capture ecran** : via `xcap` avec support multi-moniteur
- **Encodage JPEG** : encodeur d'images avec qualite configurable
- **Streaming adaptatif** : bitrate adaptatif base sur les conditions reseau
- **Presets qualite** : Basse (15 FPS), Moyenne (30 FPS), Haute (60 FPS)

### Controle distant
- **Souris** : clic, deplacement, scroll, clic droit
- **Clavier** : toutes les touches avec propagation des modifiers (Ctrl, Shift, Alt, Meta)
- **Protocole binaire** : `ControlMessage` serialise en binaire pour les frames video, JSON pour le reste

### Securite
- **CSP** : Content Security Policy stricte dans la webview Tauri
- **Chiffrement E2E** : X25519 ECDH + AES-256-GCM (module crypto)
- **PBKDF2-SHA256** : 100 000 iterations pour les mots de passe de connexion
- **Device ID persistant** : 128 bits cryptographiquement aleatoires (fichier `data/device.id`)
- **Authentification par mot de passe** : challenge/response avec hachage
- **Validation des entrees** : Device ID, SDP, ICE candidates, frames video
- **Rate limiting** : 100 messages/minute par client cote serveur
- **TLS auto-detecte** : detection automatique via `CERT_FILE` + `KEY_FILE`
- **4 serveurs STUN** : Google x2, Cloudflare, Mozilla (fallback automatique)
- **Audit trail** : logs structures JSON avec niveaux de severite
- **Sanitisation UTF-8** : troncature safe aux frontieres de caracteres

### Persistance
- **Historique connexions** : sauvegarde locale des connexions passees
- **Pairs connus** : liste des appareils avec favoris
- **Configuration** : parametres video, reseau, securite persistes

---

## Configuration

L'interface de parametres permet de configurer :

| Categorie | Options |
|-----------|---------|
| Video | Codec (H264/JPEG), framerate (15-60), bitrate (1000-10000 kbps), qualite JPEG |
| Reseau | URL serveur signalement, serveurs STUN |
| Performance | Acceleration materielle, mode faible latence, bitrate adaptatif |
| Securite | Mot de passe connexion, chiffrement E2E |

---

## Tests

```bash
# Tests unitaires Rust (50 tests)
cd client && cargo test --lib

# Tests d'integration (8 tests)
cargo test --test integration_test

# Tests de securite (8 tests)
cargo test --test security_tests

# Tests connexion (4 tests)
cargo test --test integration_connect_request

# Tests serveur Go
cd ../server && go test ./internal/signaling/... -v
```

**70 tests, 0 echecs, 0 warnings de compilation.**

---

## Statistiques du projet

| Metrique | Valeur |
|----------|--------|
| Lignes de code | ~10,200 |
| Rust | 6,560 lignes (16 fichiers) |
| Vue.js | 2,230 lignes (5 composants) |
| Go | 1,400 lignes (6 fichiers) |
| Tests | 70 (50 unit + 20 integration/securite) |
| Taille exe | ~32 MB (client + serveur embarque) |

---

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - Architecture technique detaillee
- [Developpement](docs/DEVELOPMENT.md) - Guide de developpement
- [Contribution](docs/CONTRIBUTING.md) - Guide de contribution

---

## License

MIT License

---

## Utilisation sans VPS (P2P auto-heberge)

1. **PC A** (hote) : extraire le ZIP, ouvrir port 9000 TCP, lancer `LANCER-GHOSTHANDDESK.bat`
2. **PC B** : meme ZIP, dans Parametres → URL : `ws://<IP_PC_A>:9000/ws`
3. Entrer le Device ID du PC A → Connecter

## Avec VPS public

Lancer le serveur avec `DISABLE_ORIGIN_CHECK=true`.
Configurer l'URL : `wss://votre-vps.example.com/ws`

---

**Built with Rust, Go, Tauri and Vue.js**