# 📝 Changelog - GhostHandDesk

Tous les changements notables de ce projet sont documentés dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/),
et ce projet adhère au [Semantic Versioning](https://semver.org/lang/fr/).

---

## [0.5.3] - 2026-07-02

### 🔒 Sécurité (CRITIQUE) — Chiffrement E2E authentifié de bout en bout

Depuis le mode relais VPS (v0.5.0), la clé de session E2E était capturée par valeur
au démarrage du streamer et jamais rafraîchie après le handshake : **le chiffrement
ne s'activait jamais** et l'écran, les frappes clavier, le presse-papier et les
fichiers transitaient **en clair** par le serveur relais. Cette version corrige la
faille de fond.

#### Ajouté
- **Empreinte de session (SAS)** : `crypto::session_fingerprint()` — chaîne
  comparable de 64 bits (`XXXX-XXXX-XXXX-XXXX`) affichée des deux côtés pour
  authentifier la session hors-bande, même sans mot de passe. Exposée via la
  commande `get_session_fingerprint` + l'événement `ghosthand-session-secure` +
  un bandeau UI (`App.vue`).
- **Dérivation de clé authentifiée** : `crypto::derive_session_key()` (HKDF-SHA256)
  lie le secret ECDH X25519 au secret dérivé du mot de passe → un relais malveillant
  ne peut plus intercepter la session (MITM) quand un mot de passe est défini.
- **Scellement applicatif** : `crypto::seal_frame()` / `open_frame()`
  (`[0xE2][nonce(12)][ciphertext]`) appliqués à **tout** le canal de contrôle.

#### Modifié
- **Clé de session vive** : `SessionKeyHandle` partagé, relu à chaque trame — le
  chiffrement AES-256-GCM s'active dès la fin du handshake.
- **Chiffrement obligatoire** : le streamer n'émet **jamais** en clair (trames
  écartées tant que la clé n'est pas prête, plus de fallback). L'hôte **rejette**
  tout message de contrôle en clair une fois la clé active (anti-injection relais).
  Réassemblage des fragments **avant** déchiffrement.
- **Device ID** : régénéré avec 128 bits d'entropie via `ring::SystemRandom`
  (`GHD-` + 32 hex) — correction d'une régression (était timestamp + u32).
- **Injection clavier** : `KeyboardEvent::Type` borné (8 KiB) et filtré (rejet des
  caractères de contrôle) — ne contourne plus la whitelist de touches.
- **Réception de fichiers** : `FileTransferManager::unique_path()` suffixe
  `nom (n).ext` — plus d'écrasement silencieux.

#### Sécurité
- **8 findings corrigés** (audit `AUDIT_REPORT_2026-07-02.md`) : F1 trafic contrôle
  en clair, F2 KEx non authentifié (MITM), F3 vidéo « au mieux », F4 entropie Device
  ID, F5 `Type` non filtré, F6 écrasement fichiers, F7 `STATS_TOKEN` (déjà défini
  sur le VPS), F8 capabilities shell (observation).

#### Tests
- 53 tests unitaires verts (dont `test_derive_session_key_symmetry_and_binding`,
  `test_seal_open_frame_roundtrip`, `test_session_fingerprint`).
- `cargo check` lib + Tauri OK, `go build ./...` OK, build Tauri release complet OK,
  smoke-test du portable OK.

#### Notes de migration
⚠️ Le chiffrement est désormais **obligatoire** en mode relais : les deux postes
doivent être en v0.5.3. Sans mot de passe, **comparez l'empreinte de session**
affichée des deux côtés pour authentifier la connexion.

---

## [0.2.1] - 2026-02-08 (En cours - Phase 1)

### 🔴 Corrections Critiques (Phase 1 - Bugs Bloquants)

#### Ajouté
- **Tests d'intégration ConnectRequest** : 4 nouveaux tests pour valider la sérialisation JSON
  - `test_connect_request_serialization` : Test format avec/sans password
  - `test_connect_request_deserialization` : Test parsing JSON serveur
  - `test_connect_request_format_compatibility` : Test compatibilité Rust/Go
  - `test_connect_request_message_structure` : Test structure du message
  - Fichier : `client/tests/integration_connect_request.rs`

- **Validation Device ID stricte** : Protection contre injection et DoS (BUG-004, VULN-002)
  - Fonction `validateDeviceID()` : Validation longueur (5-64 chars), caractères alphanumériques + tirets
  - Fonction `validatePassword()` : Validation longueur max 128 chars
  - Fonction `validateTargetID()` : Alias pour validateDeviceID
  - 16 tests unitaires couvrant cas valides et invalides
  - Fichiers : `server/internal/signaling/validation.go`, `validation_test.go`

- **Logs de diagnostic ConnectRequest** : Ajout de logs DEBUG pour tracer le flux
  - Log client AVANT envoi du ConnectRequest avec contenu du message
  - Format : `📤 [CLIENT] AVANT ENVOI ConnectRequest vers {target} | Message: {...}`
  - Fichier : `client/src/network.rs:669`

#### Modifié
- **Réduction MaxMessageSize** : 10 MB → 1 MB (VULN-001 - Protection DoS)
  - `MaxMessageSize = 1 * 1024 * 1024` (1 MB)
  - `MaxSDPSize = 100 * 1024` (100 KB pour SDP)
  - `MaxICESize = 1024` (1 KB pour ICE candidates)
  - Fichier : `server/internal/signaling/handler.go:16-21`

- **Validation taille SDP/ICE** : Vérification stricte dans handlers
  - `handleOffer()` : Rejette SDP > MaxSDPSize + envoie ACK erreur
  - `handleAnswer()` : Rejette SDP > MaxSDPSize + envoie ACK erreur
  - `handleIceCandidate()` : Rejette ICE > MaxICESize + envoie ACK erreur
  - Fichier : `server/internal/signaling/hub.go:292,327,361`

- **Validation Device ID dans handler** : Intégration validation avant création Client
  - Vérification AVANT création du Client dans le hub
  - Envoi message d'erreur structuré au client si Device ID invalide
  - Fermeture de connexion propre avec message d'erreur
  - Fichier : `server/internal/signaling/handler.go:96-119`

#### Sécurité
- **2 vulnérabilités corrigées** :
  - VULN-001 : Taille message WebSocket excessive (10MB → 1MB)
  - VULN-002 : Validation Device ID manquante (injection/DoS)
- **1 bug de sécurité corrigé** :
  - BUG-004 : Device ID non validé côté serveur

#### Tests
- **Client Rust** : +4 tests (total : 58 tests ✅)
- **Serveur Go** : +16 tests (total : 16 tests ✅)
- **Total projet** : 74 tests passants (100%)

### 🟡 Optimisations Performance (Phase 2)

#### Ajouté
- **Capture d'écran asynchrone** : Méthode `capture_async()` non-bloquante (PERF-001)
  - Trait `ScreenCapturer` avec support `#[async_trait]`
  - Conversion image dans `tokio::task::spawn_blocking` pour libérer le runtime
  - Libération rapide du mutex → meilleure réactivité système
  - Fichiers : `client/src/screen_capture.rs`, `client/src/streaming.rs`

- **Tests de performance** : 3 nouveaux tests pour mesurer FPS
  - `test_async_capture` : Test basique
  - `test_async_capture_performance` : Benchmark 30 frames async
  - `test_sync_capture_performance` : Benchmark comparatif sync

#### Performance
- **Mode debug** : ~5.5 FPS (sync et async identiques)
- **Mode release** : ~27.5 FPS (amélioration 5x vs debug)
- **Réactivité** : Mutex libéré plus rapidement → système plus fluide

#### Notes
- Goulot d'étranglement : `xcap::capture_image()` (Windows GDI)
- FPS limité par hardware et résolution d'écran
- 27 FPS suffisant pour utilisation bureau fluide
- Amélioration future possible avec DXGI

### 📝 Notes de développement
- Phase 1 : ✅ COMPLÉTÉE - Bugs bloquants (3/3 corrections)
- Phase 2 : ✅ COMPLÉTÉE - Optimisations performance (capture async)
- Phase 3 à venir : Support multi-moniteur UI
- Phase 4 à venir : Authentification TLS mutuelle

---

## [0.2.0] - 2026-02-07

### 🔒 Sécurité (CRITIQUE)

#### Ajouté
- **TLS Obligatoire** : Le serveur de signaling force maintenant HTTPS/WSS en production
  - Auto-génération de certificats auto-signés pour développement
  - Validation stricte des certificats
  - Variable `REQUIRE_TLS` (défaut: `true`)
  - Variable `AUTO_GENERATE_CERTS` pour génération automatique

- **Whitelist Touches Système** : Protection contre les actions dangereuses
  - Blocage touches Windows/Meta seules
  - Blocage combinaisons : Win+R, Win+L, Ctrl+Alt+Del, Alt+F4
  - Audit logging des tentatives bloquées
  - Tests unitaires complets (95% coverage)

- **Validation Stricte Entrées** : Protection anti-injection/XSS
  - Validation Device ID (format, longueur, caractères autorisés)
  - Validation SDP (taille max 100KB, format valide)
  - Validation ICE candidates (longueur max 512 chars)
  - Validation passwords (pas de null bytes, max 128 chars)
  - Sanitization pour logs (prévention log injection)

- **Rate Limiting Client** : Protection contre abus
  - Limite connexions : 5 par minute
  - Limite messages : 100 par minute
  - Fenêtre glissante configurable
  - Tests stress (10k requêtes)

#### Modifié
- `server/internal/config/config.go` : Ajout champs `RequireTLS` et `AutoGenerateCerts`
- `server/cmd/signaling/main.go` : Fonction `generateSelfSignedCert()` avec ECDSA P-256
- `client/src/input_control.rs` : Fonction `is_key_blocked()` avec whitelist
- `client/src/error.rs` : Ajout types `Validation`, `RateLimit`, `Internal`

#### Nouveau
- `client/src/validation.rs` : Module complet de validation
- `client/tests/security_tests.rs` : 15 tests de sécurité + 2 stress tests
- `scripts/generate-certs.bat` : Script Windows génération certificats
- `scripts/generate-certs.sh` : Script Linux/macOS génération certificats

---

### ⚡ Performance

#### Ajouté
- **Compression JPEG Adaptative** : Qualité dynamique basée sur latence réseau
  - Méthode `ImageEncoder::set_quality(u8)`
  - Qualité par défaut : 85 (au lieu de 80)
  - Tests unitaires

- **Adaptive Bitrate Streaming** : Ajustement automatique selon conditions réseau
  - Nouveau module `client/src/adaptive_bitrate.rs` (500+ lignes)
  - Struct `AdaptiveBitrateController` avec historique RTT/packet loss
  - Facteurs ajustement : `degradation_factor` (0.85), `improvement_factor` (1.05)
  - Seuils configurables : RTT (50ms-150ms), packet loss (5%)
  - Statistiques détaillées : `AdaptiveBitrateStats`
  - 8 tests unitaires complets

#### Modifié
- `client/src/video_encoder.rs` : Ajout méthodes `set_quality()` et `get_quality()`
- `client/src/lib.rs` : Export `AdaptiveBitrateController`
- `client/Cargo.toml` : Qualité JPEG initiale 85→85

---

### 📝 Robustesse

#### Ajouté
- **Rotation Automatique Logs** : Nettoyage automatique logs anciens
  - Rotation à 10 MB (inchangé)
  - **Nouveau** : Suppression automatique logs >30 jours
  - Fonction `AuditLogger::cleanup_old_logs()`
  - Logs archivés : `audit_<timestamp>.jsonl`
  - Tests unitaires

#### Modifié
- `client/src/audit.rs` : Fonction `rotate_log()` appelle `cleanup_old_logs()`
- `client/Cargo.toml` : Ajout `tracing-appender = "0.2"`

---

### 📚 Documentation

#### Nouveau
- `MIGRATION.md` : Guide complet de migration v0.1.0 → v0.2.0
  - 9 sections détaillées
  - Procédure pas-à-pas
  - Résolution de problèmes
  - Tableaux comparatifs performance

- `CHANGELOG.md` : Ce fichier
  - Format Keep a Changelog
  - Versioning sémantique

#### Modifié
- `README.md` : Mise à jour badges version (0.1.0 → 0.2.0)

---

### 🔧 Maintenance

#### Modifié
- Tous les modules : Amélioration logs (`debug!`, `info!`, `warn!`, `error!`)
- `client/src/network.rs` : Constantes validation ajoutées
- Tests : +25 tests unitaires (+300% coverage sur nouveaux modules)

---

### 🐛 Corrections

#### Résolu
- **Tâche #1** : Serveur acceptait HTTP en production (CRITIQUE)
- **Tâche #2** : Touches système (Win+R, etc.) non bloquées (HAUTE)
- **Tâche #3** : Logs croissaient indéfiniment (MOYENNE)
- **Tâche #4** : Qualité JPEG fixe quelle que soit latence (BASSE)
- **Tâche #7** : Pas de validation entrées utilisateur (HAUTE)

---

## [0.1.0] - 2025-XX-XX

### Ajouté
- Implémentation initiale bureau à distance P2P
- Client Tauri (Rust + Vue.js)
- Serveur signaling Go
- Chiffrement E2E (X25519 + AES-256-GCM)
- Streaming vidéo JPEG/H.264
- Contrôle input distant (souris/clavier)
- WebRTC avec ICE/STUN
- Audit trail JSON structuré
- Storage persistant (historique, favoris)
- Rate limiting serveur (100 msg/min)

---

## Notes de Version

### Breaking Changes v0.2.0

⚠️ **TLS Obligatoire** : Les clients doivent se connecter en WSS (WebSocket Secure).
- Configurer `REQUIRE_TLS=false` uniquement pour développement local
- Générer certificats avec `scripts/generate-certs.bat` ou `.sh`

⚠️ **Nouvelles Validations** : Les Device IDs doivent respecter le format strict.
- Longueur : 5-64 caractères
- Caractères autorisés : `a-z`, `A-Z`, `0-9`, `-`

### Compatibilité

- **Rust** : 1.70+ (inchangé)
- **Go** : 1.21+ (inchangé)
- **Node.js** : 18+ (inchangé)
- **Tauri** : 2.0+ (inchangé)

### Taille Binaires

| Composant | v0.1.0 | v0.2.0 | Différence |
|-----------|--------|--------|------------|
| Client (release) | ~8.2 MB | ~8.5 MB | +3.7% |
| Serveur | ~6.1 MB | ~6.3 MB | +3.3% |

Augmentation due aux nouvelles fonctionnalités de sécurité et validation.

---

## Prochaines Versions

### [0.3.0] - Prévu Q2 2026

#### Planifié
- [ ] Multi-monitor support
- [ ] Clipboard sharing sécurisé
- [ ] File transfer chiffré
- [ ] Audio streaming
- [ ] Authentification 2FA
- [ ] Compression H.265 (HEVC)
- [ ] Mobile support (Android/iOS)

#### En Considération
- [ ] Session recording
- [ ] Watermarking
- [ ] Permissions granulaires
- [ ] LDAP/SSO integration
- [ ] Metrics dashboard (Grafana)

---

## Contributeurs

- **Claude Sonnet 4.5** - Implémentation complète v0.2.0
- **Équipe GhostHandDesk** - Architecture & design

---

## Liens

- [Documentation](README.md)
- [Guide Migration](MIGRATION.md)
- [Issues](https://github.com/heiphaistos44-crypto/GhostHandDesk/issues)
- [Releases](https://github.com/heiphaistos44-crypto/GhostHandDesk/releases)
