# Release Notes — GhostHandDesk

---

## v0.5.3 — 2026-07-02 — Chiffrement E2E authentifié

### 🔒 Sécurité — correctif critique
Depuis le mode relais (v0.5.0), la clé de session E2E n'était jamais réellement
activée : écran, frappes clavier, presse-papier et fichiers transitaient **en
clair** par le serveur relais. Cette version rétablit un chiffrement de bout en
bout réel et authentifié.

- **Clé de session vive** : le chiffrement AES-256-GCM s'active dès la fin du
  handshake X25519 et couvre **tout** le canal (vidéo, souris, clavier,
  presse-papier, chat, fichiers).
- **Échange de clés authentifié** : la clé de session est liée au mot de passe
  (HKDF-SHA256) → un relais malveillant ne peut plus intercepter la session (MITM).
- **Chiffrement obligatoire** : plus aucune trame en clair ; l'hôte rejette tout
  input non chiffré.
- **Empreinte de session (SAS)** : une empreinte comparable `XXXX-XXXX-XXXX-XXXX`
  est affichée des deux côtés pour authentifier la session même sans mot de passe.

### Autres correctifs
- **Device ID** : 128 bits d'entropie via CSPRNG (correction de régression).
- **Injection clavier** : `Type` borné et filtré (ne contourne plus la whitelist).
- **Réception de fichiers** : plus d'écrasement silencieux (suffixe `nom (n).ext`).

### Qualité
- 53 tests unitaires verts, build Tauri release complet, smoke-test OK.
- Serveur Go inchangé (correctifs 100 % côté client) — aucun redéploiement requis.

### ⚠️ Mise à jour
Les deux postes doivent être en v0.5.3. Sans mot de passe, **comparez l'empreinte
de session** affichée des deux côtés pour écarter tout intercepteur.

**Recommandé** : `GhostHandDesk_0.5.3_portable_x64.exe` (exe unique, aucun prérequis).

---

## v0.2.0 — 2026-05-20

### Sécurité
- **Chiffrement E2E** : X25519 ECDH + AES-256-GCM (implémentation Rust pure)
- **PBKDF2-SHA256** : 100 000 itérations pour les mots de passe de connexion
- **Device ID persistant** : 128 bits crypto-aléatoires, sauvegardés dans `data/device.id`
- **TLS auto-détecté** : `CERT_FILE` + `KEY_FILE` → `wss://` activé automatiquement
- **CSP Tauri** : Content Security Policy stricte dans la webview
- **Rate limiting** : 100 messages/minute par client côté serveur Go
- **Validation stricte** : Device ID, SDP, ICE candidates, frames vidéo

### Connectivité Internet
- **4 serveurs STUN** : Google×2, Cloudflare, Mozilla (fallback automatique)
- **Paramètres réseau persistés** : `data/settings.json` via `settings_commands.rs`
- **`DISABLE_ORIGIN_CHECK=true`** pour déploiement VPS public (CORS)
- **Adaptive bitrate** : ajustement automatique selon les conditions réseau

### Nouvelles fonctionnalités
- **Synchronisation presse-papier** : copier/coller entre hôte et contrôleur
- **Transfert de fichiers** : envoi de fichiers pendant la session
- **Chat intégré** : messagerie texte pendant la connexion
- **Icône system tray** : GhostHandDesk en arrière-plan avec menu contextuel
- **Multi-moniteur** : basculement entre écrans de l'hôte
- **Sélecteur de résolution** : presets configurables (Basse/Moyenne/Haute/Personnalisée)
- **Popup d'acceptation** : l'hôte valide ou rejette chaque demande de connexion

### Qualité
- **54/54 tests passés** (unitaires + intégration + sécurité)
- Fix latence souris (précision des coordonnées normalisées)
- Fix clics droits et scroll

### Dépendances npm (UI)
- `picomatch` → version sécurisée (ReDoS fix)
- `postcss` → >=8.5.10 (XSS fix)
- `rollup` → >=4.58.0 (path traversal fix)

---

## v0.1.0 — 2026-05-18

### Version initiale
- Capture d'écran via `xcap` (Rust, multi-moniteur)
- Streaming JPEG avec qualité configurable
- Contrôle souris (clic, déplacement, scroll, clic droit)
- Contrôle clavier (toutes touches + modifiers)
- Serveur de signalement Go embarqué via `include_bytes!`
- Auto-découverte LAN (ports 9000–9004)
- Preview local de l'écran partagé
- WebRTC P2P (DTLS-SRTP natif)
- Interface Vue.js 3 + TypeScript
- Historique des connexions + pairs connus
- 70 tests (50 unitaires + 20 intégration/sécurité)
