# Interface Tauri - GhostHandDesk

Interface utilisateur moderne pour GhostHandDesk, construite avec Tauri 2.0 + Vue 3 + TypeScript.

## 🏗️ Architecture

```
client/
├── src/               # Bibliothèque Rust (core)
├── src-tauri/         # Backend Tauri
│   ├── src/main.rs   # Point d'entrée Tauri
│   └── Cargo.toml    # Dépendances Tauri
└── ui/                # Frontend Vue 3
    ├── src/
    │   ├── App.vue              # Composant principal
    │   ├── components/
    │   │   ├── ConnectDialog.vue    # Dialog de connexion
    │   │   ├── RemoteViewer.vue     # Viewer vidéo + contrôle
    │   │   └── SettingsPanel.vue    # Panneau paramètres
    │   └── main.ts              # Point d'entrée TS
    ├── index.html
    ├── package.json
    └── vite.config.ts
```

## 🚀 Lancement

### Mode développement

```bash
cd client

# Lancer le backend Tauri + frontend Vite
cargo tauri dev
```

**Sortie attendue :**
- Serveur Vite démarre sur http://localhost:5173
- Backend Tauri compile et lance l'application
- Fenêtre Tauri s'ouvre avec l'interface Vue

### Mode production

```bash
cd client

# Compiler l'application
cargo tauri build

# Binaire dans : target/release/
```

## 🎨 Composants

### App.vue
- Layout principal
- Gestion d'état (connecté/déconnecté)
- Affichage Device ID
- Indicateur de connexion

### ConnectDialog.vue
- Formulaire de connexion
- Input Device ID cible
- Input mot de passe (optionnel)
- Gestion d'erreurs
- Actions rapides (Aide, Paramètres, À propos)

### RemoteViewer.vue
- Canvas pour streaming vidéo
- Toolbar (disconnect, fullscreen, screenshot, qualité)
- Gestion événements souris (move, click, scroll)
- Gestion événements clavier (keydown, keyup)
- Indicateurs FPS et latence
- Overlay de connexion

### SettingsPanel.vue
- Paramètres vidéo (codec, framerate, bitrate, qualité)
- Paramètres réseau (serveur signaling, STUN)
- Paramètres performance (accél. matérielle, faible latence)
- Paramètres interface (FPS counter, latence, clipboard)
- Paramètres sécurité (mot de passe, chiffrement)

## 🔌 API Tauri (commandes invoke)

### `get_device_id() -> String`
Récupère le Device ID de l'appareil actuel.

```typescript
import { invoke } from '@tauri-apps/api/core';

const deviceId = await invoke<string>('get_device_id');
console.log('Device ID:', deviceId); // GHD-abc123def456
```

### `connect_to_device(targetId: string, password?: string) -> Result<void, string>`
Établit une connexion WebRTC avec un appareil distant.

```typescript
try {
  await invoke('connect_to_device', {
    targetId: 'GHD-target123',
    password: 'mypassword' // optionnel
  });
  console.log('Connecté !');
} catch (error) {
  console.error('Erreur:', error);
}
```

### `disconnect() -> Result<void, string>`
Ferme la connexion active.

```typescript
await invoke('disconnect');
```

### `send_mouse_event(event: MouseEvent) -> Result<void, string>`
Envoie un événement souris à l'appareil distant.

```typescript
await invoke('send_mouse_event', {
  event: {
    x: 100,
    y: 200,
    button: 'left', // 'left' | 'right' | 'middle' | 'none'
    type: 'press',  // 'press' | 'release' | 'move' | 'scroll'
    delta: 0        // Pour scroll seulement
  }
});
```

### `send_keyboard_event(event: KeyboardEvent) -> Result<void, string>`
Envoie un événement clavier à l'appareil distant.

```typescript
await invoke('send_keyboard_event', {
  event: {
    key: 'a',
    code: 'KeyA',
    type: 'press', // 'press' | 'release'
    modifiers: {
      ctrl: false,
      shift: false,
      alt: false,
      meta: false
    }
  }
});
```

### `get_config() -> Result<Config, string>`
Récupère la configuration actuelle.

```typescript
const config = await invoke('get_config');
console.log('Serveur:', config.server_url);
```

### `update_config(config: Config) -> Result<void, string>`
Met à jour la configuration.

```typescript
await invoke('update_config', {
  new_config: {
    server_url: 'wss://myserver.com/ws',
    stun_servers: ['stun:stun.l.google.com:19302'],
    video_config: {
      codec: 'H264',
      framerate: 30,
      bitrate: 4000,
      quality: 80
    }
  }
});
```

## 📡 Événements (à implémenter)

### `video-frame`
Émis quand une nouvelle frame vidéo est reçue.

```typescript
import { listen } from '@tauri-apps/api/event';

await listen('video-frame', (event) => {
  const { data, width, height, timestamp } = event.payload;
  // Dessiner sur canvas
});
```

## 🎨 Thème

Variables CSS personnalisables dans `App.vue` :

```css
--color-bg: #1e1e1e;
--color-bg-secondary: #2d2d30;
--color-border: #3e3e42;
--color-text: #ffffff;
--color-text-secondary: #9d9d9d;
--color-accent: #0e639c;
--color-success: #4ec9b0;
--color-error: #f88;
```

## 🐛 Débogage

### Activer les DevTools

```bash
# En mode dev, ouvrir les DevTools : F12 ou Ctrl+Shift+I
```

### Logs backend Tauri

Les `println!` dans `main.rs` s'affichent dans la console.

```rust
println!("[TAURI] Message de debug");
```

### Logs frontend Vue

```typescript
console.log('Message de debug');
```

## 📦 Build

### Binaires produits

**Windows :**
- `target/release/ghosthanddesk-tauri.exe`
- Installer MSI : `target/release/bundle/msi/GhostHandDesk_0.1.0_x64_en-US.msi`

**Linux :**
- `target/release/ghosthanddesk-tauri`
- Package Deb : `target/release/bundle/deb/ghosthanddesk_0.1.0_amd64.deb`
- Package AppImage : `target/release/bundle/appimage/ghosthanddesk_0.1.0_amd64.AppImage`

**macOS :**
- App Bundle : `target/release/bundle/macos/GhostHandDesk.app`
- DMG : `target/release/bundle/dmg/GhostHandDesk_0.1.0_x64.dmg`

## 🔧 Développement

### Structure de fichier

```typescript
// ui/src/types.ts (à créer)
export interface MouseEvent {
  x: number;
  y: number;
  button: 'left' | 'right' | 'middle' | 'none';
  type: 'press' | 'release' | 'move' | 'scroll';
  delta?: number;
}

export interface KeyboardEvent {
  key: string;
  code: string;
  type: 'press' | 'release';
  modifiers: {
    ctrl: boolean;
    shift: boolean;
    alt: boolean;
    meta: boolean;
  };
}
```

### Hot Reload

Le hot reload fonctionne pour :
- ✅ Frontend Vue (modifications CSS/HTML/TS)
- ❌ Backend Rust (nécessite recompilation)

Pour recharger le backend :
1. Arrêter `cargo tauri dev` (Ctrl+C)
2. Relancer `cargo tauri dev`

## 📝 TODO

- [ ] Implémenter l'émission d'événements `video-frame` depuis le backend
- [ ] Connecter les événements souris/clavier au WebRTC data channel
- [ ] Implémenter la synchronisation du presse-papiers
- [ ] Ajouter la gestion multi-moniteurs côté remote
- [ ] Implémenter le transfert de fichiers
- [ ] Ajouter l'audio streaming

## 🆘 Troubleshooting

### Erreur "Failed to resolve entry"

**Cause :** Le frontend Vite n'a pas démarré.

**Solution :**
```bash
cd ui
npm run dev  # Tester le frontend séparément
```

### Erreur "Could not find `ghost_hand_client`"

**Cause :** Le client n'est pas construit comme lib.

**Solution :** Vérifier que `client/src/lib.rs` existe et que `Cargo.toml` a :
```toml
[lib]
name = "ghost_hand_client"
path = "src/lib.rs"
```

### L'interface ne se charge pas

**Cause :** Port 5173 déjà utilisé.

**Solution :** Changer le port dans `vite.config.ts` et `tauri.conf.json`.

## 📚 Ressources

- [Documentation Tauri](https://v2.tauri.app/)
- [Guide Vue 3](https://vuejs.org/guide/)
- [API Tauri](https://v2.tauri.app/reference/javascript/api/)
