# Installation de FFmpeg pour GhostHandDesk

## Pourquoi FFmpeg ?

FFmpeg est utilisé pour l'encodage vidéo H.264/H.265 avec faible latence. Sans FFmpeg, le client utilisera l'encodeur JPEG (fallback), qui est moins efficace mais fonctionnel.

## Installation par plateforme

### Windows

**Option 1 : Via Chocolatey (recommandé)**
```bash
choco install ffmpeg
```

**Option 2 : Téléchargement manuel**
1. Télécharger depuis : https://www.gyan.dev/ffmpeg/builds/
2. Choisir "ffmpeg-release-essentials.zip"
3. Extraire dans `C:\ffmpeg`
4. Ajouter `C:\ffmpeg\bin` au PATH système

**Vérification:**
```bash
ffmpeg -version
```

### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev pkg-config
```

### macOS

```bash
brew install ffmpeg pkg-config
```

## Compilation avec FFmpeg

Une fois FFmpeg installé :

```bash
cd client
cargo build --features ffmpeg
```

## Vérifier l'accélération matérielle

### NVIDIA (NVENC)
```bash
ffmpeg -encoders | grep nvenc
```

Si disponible, modifier `video_encoder.rs` pour utiliser `h264_nvenc` au lieu de `libx264`.

### Intel (QuickSync)
```bash
ffmpeg -encoders | grep qsv
```

### AMD (AMF)
```bash
ffmpeg -encoders | grep amf
```

## Sans FFmpeg

Le client fonctionne sans FFmpeg en utilisant l'encodeur JPEG :

```bash
cargo build  # Sans feature ffmpeg
```

**Limitations :**
- Pas de compression H.264
- Bande passante plus élevée
- Qualité légèrement inférieure

## Benchmarks

| Encodeur | CPU Usage | Bande passante | Latency |
|----------|-----------|----------------|---------|
| H.264 (FFmpeg software) | ~15% | 2-4 Mbps | < 50ms |
| H.264 (NVENC) | ~5% | 2-4 Mbps | < 30ms |
| JPEG (fallback) | ~10% | 10-20 Mbps | < 40ms |

## Troubleshooting

### Erreur "ffmpeg-sys-next build failed"

**Cause :** FFmpeg non installé ou non dans le PATH.

**Solution :**
1. Vérifier : `ffmpeg -version`
2. Vérifier PATH : `echo %PATH%` (Windows) ou `echo $PATH` (Linux/macOS)
3. Redémarrer le terminal après installation

### Erreur de linking

**Windows :** Installer Visual Studio Build Tools
**Linux :** Installer `build-essential`
**macOS :** Installer Xcode Command Line Tools

### Performance faible

1. Activer l'accélération matérielle (NVENC, QSV, etc.)
2. Utiliser un preset plus rapide : `ultrafast` (déjà configuré)
3. Réduire la résolution ou le bitrate dans la config
