# Architecture GhostHandDesk

## Vue d'Ensemble

```
┌─────────────┐      WebSocket       ┌──────────┐      WebSocket      ┌─────────────┐
│  Client A   │◄─────Signaling──────►│  Serveur │◄────Signaling──────►│  Client B   │
│ (Contrôle)  │                       │   (Go)   │                      │ (Contrôlé)  │
└──────┬──────┘                       └──────────┘                      └──────┬──────┘
       │                                                                        │
       │                          WebRTC P2P                                   │
       └───────────────────────────────────────────────────────────────────────┘
              (Data Channel: Video Stream + Input Control)
```

## Modules Client (Rust)

### network.rs
- Gestion WebRTC (peer connections, ICE, SDP)
- Signaling WebSocket
- Session management

### streaming.rs
- `Streamer`: Capture → Encode → Send
- `Receiver`: Receive → Decode → Display
- `InputHandler`: Input events → Network

### storage.rs
- Persistance JSON
- Historique connexions
- Pairs favoris

### audit.rs
- Logging structuré
- Rotation automatique
- Format JSONL

## Serveur (Go)

### hub.go
- Hub central
- Routing messages
- Gestion clients connectés

### handler.go
- WebSocket upgrade
- CORS validation
- Rate limiting

## Flux de Données

1. **Capture** : `screen_capture.rs` → Frame RGB/BGRA
2. **Encodage** : `video_encoder.rs` → JPEG compressé
3. **Protocole** : Format binaire optimisé (24 bytes header + data)
4. **Transmission** : WebRTC Data Channel
5. **Décodage** : Browser Image API
6. **Affichage** : Canvas HTML5

