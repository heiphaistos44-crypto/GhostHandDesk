# GhostHandDesk

<div align="center">

**Open-source remote desktop application - A modern alternative to TeamViewer, AnyDesk, and RustDesk**

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

</div>

## Features

- **Cross-platform**: Windows, Linux, macOS support
- **High performance**: Rust-powered with hardware-accelerated video encoding
- **Secure**: End-to-end encryption using AES-256-GCM
- **WebRTC**: Peer-to-peer connections with NAT traversal (STUN/TURN)
- **Simple deployment**: Single portable executable
- **Open source**: Fully auditable and customizable

## Architecture

### Client (Rust)
- **Screen Capture**: Multi-monitor support using `xcap`
- **Video Encoding**: H.264/H.265 with hardware acceleration
- **Input Control**: Cross-platform keyboard/mouse simulation
- **Network**: WebRTC for P2P streaming
- **Security**: End-to-end encryption with `ring`
- **UI**: Modern interface with Tauri (planned)

### Server (Go) - Coming Soon
- Signaling server for device discovery
- TURN relay for NAT traversal
- WebSocket-based communication

## Getting Started

### Prerequisites

- Rust 1.70 or later
- FFmpeg (optional, for H.264 encoding)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/heiphaistos44-crypto/GhostHandDesk.git
cd GhostHandDesk

# Build the client
cd client
cargo build --release

# Run the client
cargo run --release
```

### Configuration

Create a `config.json` file:

```json
{
  "server_url": "wss://signal.your-domain.com:8443",
  "stun_servers": [
    "stun:stun.l.google.com:19302"
  ],
  "video_config": {
    "framerate": 30,
    "codec": "H264",
    "bitrate": 4000,
    "hardware_acceleration": true
  },
  "security_config": {
    "e2e_encryption": true,
    "require_auth": true
  }
}
```

## Development Status

- [x] Project structure
- [x] Screen capture (multi-monitor)
- [x] Video encoding (JPEG, H.264 planned)
- [x] Input control (keyboard/mouse)
- [x] Cryptography (AES-256-GCM)
- [x] Network stack (WebSocket signaling)
- [ ] WebRTC implementation
- [ ] Tauri UI
- [ ] Go signaling server
- [ ] TURN relay server
- [ ] File transfer
- [ ] Clipboard sync
- [ ] Audio streaming

## Technology Stack

| Component | Technology |
|-----------|-----------|
| Client Language | Rust |
| UI Framework | Tauri |
| Screen Capture | xcap |
| Video Encoding | FFmpeg (H.264/H.265) |
| Network Protocol | WebRTC |
| Signaling | WebSocket |
| Encryption | AES-256-GCM (ring) |
| Server Language | Go (planned) |

## Security

GhostHandDesk takes security seriously:

- **E2E Encryption**: All data is encrypted end-to-end using AES-256-GCM
- **TLS**: Signaling server connections use TLS 1.3
- **Authentication**: Password-based authentication with secure hashing
- **No data collection**: Your data stays on your devices

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting PRs.

## Roadmap

### Phase 1: Core Functionality (Current)
- Basic screen capture
- Video encoding
- Network stack
- Input control

### Phase 2: WebRTC & UI
- Complete WebRTC implementation
- Tauri-based UI
- Connection management

### Phase 3: Server Infrastructure
- Go signaling server
- TURN relay server
- Docker deployment

### Phase 4: Advanced Features
- File transfer
- Clipboard synchronization
- Audio streaming
- Mobile apps

## License

This project is dual-licensed under:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Acknowledgments

- Inspired by [RustDesk](https://github.com/rustdesk/rustdesk)
- Built with amazing Rust crates from the community

## Contact

- GitHub Issues: [Report bugs or request features](https://github.com/heiphaistos44-crypto/GhostHandDesk/issues)

---

Made with ❤️ by the GhostHandDesk community
