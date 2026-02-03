# Serveur de Signalement GhostHandDesk

Serveur WebSocket pour gérer la signalisation WebRTC entre clients GhostHandDesk.

## Prérequis

### Installation de Go

**Windows (via Chocolatey):**
```bash
choco install golang
```

**Ou télécharger depuis:** https://go.dev/dl/

## Installation

1. **Télécharger les dépendances:**
```bash
cd server
go mod download
```

2. **Générer les certificats TLS (développement):**
```bash
mkdir certs
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout certs/server.key \
  -out certs/server.crt \
  -days 365 -subj "/CN=localhost"
```

3. **Configurer les variables d'environnement:**
```bash
cp .env.example .env
# Éditer .env si nécessaire
```

## Lancement

### Mode développement
```bash
go run cmd/signaling/main.go
```

### Mode production (compilation)
```bash
go build -o bin/signaling.exe cmd/signaling/main.go
./bin/signaling.exe
```

## Routes disponibles

- **WebSocket:** `wss://localhost:8443/ws`
- **Health Check:** `https://localhost:8443/health`
- **Statistiques:** `https://localhost:8443/stats`

## Architecture

```
server/
├── cmd/signaling/main.go      # Point d'entrée du serveur
├── internal/
│   ├── config/config.go       # Gestion de la configuration
│   ├── models/message.go      # Structures de messages
│   └── signaling/
│       ├── hub.go             # Hub de gestion des clients
│       └── handler.go         # Handler WebSocket
├── certs/                     # Certificats TLS
├── go.mod                     # Dépendances Go
└── .env                       # Configuration
```

## Protocole de signalisation

### Messages supportés

1. **Register** - Enregistrement d'un client
```json
{
  "type": "Register",
  "data": {
    "device_id": "GHD-abc123"
  }
}
```

2. **Offer** - Offre WebRTC
```json
{
  "type": "Offer",
  "data": {
    "from": "GHD-abc123",
    "to": "GHD-def456",
    "sdp": "v=0\r\no=- ..."
  }
}
```

3. **Answer** - Réponse WebRTC
```json
{
  "type": "Answer",
  "data": {
    "from": "GHD-def456",
    "to": "GHD-abc123",
    "sdp": "v=0\r\no=- ..."
  }
}
```

4. **IceCandidate** - Candidat ICE
```json
{
  "type": "IceCandidate",
  "data": {
    "from": "GHD-abc123",
    "to": "GHD-def456",
    "candidate": "candidate:...",
    "sdp_mid": "0",
    "sdp_mline_index": 0
  }
}
```

5. **Ping/Pong** - Keepalive
```json
{
  "type": "Ping"
}
```

## Tests

```bash
go test ./...
```

## Docker (optionnel)

```bash
docker build -t ghosthanddesk-server .
docker run -p 8443:8443 ghosthanddesk-server
```

## Sécurité

- **TLS obligatoire** en production
- Utiliser des certificats valides (Let's Encrypt)
- Configurer `CheckOrigin` dans le handler WebSocket
- Limiter `MAX_CLIENTS` selon les ressources disponibles

## Troubleshooting

### Erreur "certificate signed by unknown authority"
Utiliser des certificats auto-signés uniquement en développement. En production, utiliser Let's Encrypt ou un CA reconnu.

### Port 8443 déjà utilisé
Changer `SERVER_HOST` dans `.env` (ex: `:9443`)

### Client non enregistré
Vérifier que le premier message envoyé est bien de type `Register`
