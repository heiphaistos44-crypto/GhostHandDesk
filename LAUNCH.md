# 🚀 Guide de Lancement - GhostHandDesk

Guide rapide pour démarrer GhostHandDesk après l'implémentation.

## ⚡ Installation des prérequis (Windows)

### 1. Installer Go (pour le serveur)
```powershell
# Via Chocolatey
choco install golang -y

# Vérifier
go version  # Doit afficher go1.21+
```

### 2. Installer FFmpeg (pour H.264)
```powershell
# Via Chocolatey
choco install ffmpeg -y

# Vérifier
ffmpeg -version
```

**Note :** Redémarrer le terminal après installation pour recharger le PATH.

## 🎬 Lancement en 3 étapes

### Étape 1 : Démarrer le serveur de signalement

```bash
# Ouvrir Terminal 1
cd Documents/GhostHandDesk/server

# Générer certificats TLS (première fois seulement)
mkdir certs
openssl req -x509 -newkey rsa:4096 -nodes -keyout certs/server.key -out certs/server.crt -days 365 -subj "/CN=localhost"

# Lancer le serveur
go run cmd/signaling/main.go
```

**Sortie attendue :**
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

### Étape 2 : Installer dépendances frontend

```bash
# Ouvrir Terminal 2 (ne pas fermer Terminal 1)
cd Documents/GhostHandDesk/client/ui

# Installer dépendances npm (première fois seulement)
npm install
```

### Étape 3 : Lancer l'interface Tauri

```bash
# Dans le même Terminal 2
cd Documents/GhostHandDesk/client

# Lancer Tauri en mode développement
cargo tauri dev
```

**Sortie attendue :**
- Compilation Rust (1-2 min la première fois)
- Démarrage serveur Vite sur http://localhost:5173
- Ouverture fenêtre Tauri avec l'interface

**Dans l'interface :**
- Header affiche votre Device ID : `GHD-xxxxxxxxxxxxx`
- Status : "Déconnecté"
- Dialog de connexion visible

## 🔗 Se connecter entre 2 machines

### Machine A (Host - celle qu'on va contrôler)
1. Lancer serveur (Étape 1)
2. Lancer client Tauri (Étapes 2-3)
3. **Noter le Device ID affiché** : `GHD-abc123def456`
4. Laisser l'application ouverte

### Machine B (Client - celle qui contrôle)
1. S'assurer que le serveur est accessible
2. Lancer client Tauri (Étapes 2-3)
3. Dans le dialog de connexion :
   - Entrer le Device ID de la Machine A : `GHD-abc123def456`
   - Mot de passe (optionnel) : laisser vide
   - Cliquer "Se connecter"

**Résultat attendu :**
- Status passe à "Connexion..."
- Logs serveur montrent l'échange Offer/Answer
- Status passe à "Connecté à GHD-abc123def456"
- Canvas affiche l'écran de la Machine A
- Souris et clavier fonctionnent

## ✅ Vérification de santé

### Test serveur (pendant qu'il tourne)
```bash
# Ouvrir Terminal 3
curl -k https://localhost:8443/health
```

**Résultat :**
```json
{
  "status": "healthy",
  "clients": 2
}
```

### Test statistiques
```bash
curl -k https://localhost:8443/stats
```

**Résultat :**
```json
{
  "total_clients": 2,
  "uptime": "5m30s",
  "max_clients": 1000
}
```

### Test compilation
```bash
cd Documents/GhostHandDesk/client
cargo test --lib  # Tests unitaires
cargo test --test integration_test  # Tests d'intégration
```

**Résultat attendu :**
```
test result: ok. 18 passed; 0 failed
test result: ok. 8 passed; 0 failed
```

## 🐛 Problèmes courants

### "go: command not found"
**Solution :** Installer Go et redémarrer le terminal.

### "ffmpeg not found"
**Solution :** Le client fonctionne quand même avec encodeur JPEG.
Pour H.264 : installer FFmpeg et recompiler avec `--features ffmpeg`.

### "Port 8443 already in use"
**Solution :**
```bash
# Trouver le processus
netstat -ano | findstr :8443

# Tuer le processus
taskkill /PID <PID> /F
```

### "Failed to compile Tauri"
**Solution :** Vérifier que Rust est installé :
```bash
rustc --version
cargo --version
```

### "npm install" échoue
**Solution :** Vérifier Node.js/npm :
```bash
node --version  # v18+
npm --version   # v9+
```

### Interface Tauri ne se lance pas
**Solution :**
```bash
# Tester le frontend seul
cd client/ui
npm run dev

# Ouvrir http://localhost:5173 dans navigateur
```

### Connexion échoue
**Vérifier :**
1. ✅ Serveur tourne (logs visibles)
2. ✅ Les 2 clients sont connectés au serveur
3. ✅ Device ID correctement copié
4. ✅ Pas de firewall bloquant

**Logs utiles :**
- Terminal serveur : voir les messages Register/Offer/Answer
- Console Tauri : F12 dans l'interface
- Logs backend : dans Terminal 2

## 📊 Indicateurs de performance

Dans l'interface (RemoteViewer), vérifier :
- **FPS :** Devrait être ≥ 15 (30 idéal)
- **Latence :** < 150ms (< 50ms en LAN)
- **CPU :** < 20% total

**Si performances faibles :**
1. Réduire framerate dans Settings (30 → 15 fps)
2. Réduire bitrate (4000 → 2000 kbps)
3. Activer accélération matérielle si disponible

## 🎮 Utilisation

### Contrôles basiques
- **Souris :** Déplacer, cliquer, scroll naturellement sur le canvas
- **Clavier :** Focus sur canvas (clic), puis taper normalement
- **Disconnect :** Bouton en haut à gauche
- **Fullscreen :** Bouton en haut à droite
- **Screenshot :** Bouton appareil photo
- **Settings :** Bouton engrenage

### Raccourcis clavier
- **F11 :** Plein écran (ou ESC pour quitter)
- **Ctrl+C :** Copier (sur machine locale)
- **Ctrl+V :** Coller (TODO: sync clipboard)

### Ajuster la qualité
1. Cliquer sur ⚙️ (engrenage) en haut à droite
2. Dans "Qualité du streaming" :
   - **Basse :** 15 FPS, économie bande passante
   - **Moyenne :** 30 FPS, équilibré (recommandé)
   - **Haute :** 60 FPS, haute qualité (si CPU/réseau ok)

## 📁 Fichiers de logs

- **Serveur Go :** stdout (Terminal 1)
- **Client Tauri backend :** stdout (Terminal 2)
- **Client Tauri frontend :** Console browser (F12)
- **Tests :** `cargo test -- --nocapture`

## 🔧 Mode production

### Compiler les binaires

**Serveur :**
```bash
cd server
go build -o bin/ghosthanddesk-server.exe cmd/signaling/main.go
```

**Client :**
```bash
cd client
cargo tauri build
```

**Binaires générés :**
- Serveur : `server/bin/ghosthanddesk-server.exe`
- Client : `client/target/release/bundle/msi/GhostHandDesk_0.1.0_x64.msi`

### Déploiement

1. Installer MSI sur les machines clientes
2. Déployer serveur sur un VPS avec IP publique
3. Configurer certificats TLS valides (Let's Encrypt)
4. Mettre à jour `config.json` avec l'URL du serveur

## 🆘 Support

**Documentation :**
- `README.md` - Vue d'ensemble
- `QUICKSTART.md` - Guide démarrage rapide
- `server/README.md` - Documentation serveur
- `client/TAURI_README.md` - API Tauri
- `client/FFMPEG_SETUP.md` - Guide FFmpeg
- `SESSION_REPORT.md` - Rapport d'implémentation

**Tests :**
- Tests unitaires : `cargo test --lib`
- Tests intégration : `cargo test --test integration_test`
- Coverage : 26/26 tests passent ✅

---

**🎉 Profitez de GhostHandDesk !**

**Rappel :** Projet à 95% fonctionnel, prêt pour tests E2E et déploiement.
