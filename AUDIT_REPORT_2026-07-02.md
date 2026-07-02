# AUDIT COMPLET — GhostHandDesk v0.5.2

**Date:** 2026-07-02
**Auditeur:** Claude Code (Fable 5)
**Périmètre:** client Rust/Tauri, lib crypto/réseau/streaming/input, serveur Go de signalement, config Tauri/VPS
**Verdict:** Base solide (audits précédents bien appliqués), mais **1 faille d'architecture CRITIQUE** invalide la promesse « chiffrement E2E » depuis le passage en mode relay VPS (v0.5.0).

---

## Synthèse sévérités

| # | Sévérité | Titre | Zone |
|---|----------|-------|------|
| F1 | **CRITIQUE** | Trafic contrôle (clavier/clipboard/chat/fichiers) en clair via le VPS | streaming/main.rs |
| F2 | **CRITIQUE** | Échange de clés E2E (X25519) non authentifié → MITM par le relay | main.rs KEx |
| F3 | **HAUTE** | Vidéo chiffrée seulement « au mieux » (premières frames + fallback en clair) | streaming.rs |
| F4 | **MOYENNE** | Régression entropie Device ID (32 bits au lieu de 128) | network.rs |
| F5 | **MOYENNE** | `KeyboardEvent::Type` contourne toute la whitelist de touches | input_control.rs |
| F6 | **FAIBLE** | Réception de fichiers sans confirmation ni anti-écrasement | file_transfer.rs |
| F7 | **FAIBLE** | `/stats` public si `STATS_TOKEN` non défini | server main.go |
| F8 | **INFO** | Aucun fichier `capabilities/` Tauri (à vérifier) | src-tauri |

---

## F1 — [CRITIQUE] Le trafic de contrôle transite en clair par le VPS

**Fichiers:** `client/src-tauri/src/main.rs` (`send_mouse_event`, `send_keyboard_event`, `sync_clipboard`, `send_chat_message`, `send_file`, `change_*`) ; `client/src/network.rs` (`RelayTransport`).

Depuis la v0.5.0, le transport par défaut est **`RelayTransport`** : toutes les données sont encodées en base64 et **relayées par le serveur VPS** (`SignalMessage::relay_data` → hub Go → pair). Le WebRTC P2P (et donc DTLS-SRTP) n'est plus utilisé dans ce chemin.

Or seul le **flux vidéo** applique éventuellement la clé de session (voir F3). Tous les messages de contrôle sont envoyés via `webrtc.send_data(&bytes)` **sans aucun chiffrement applicatif** :

```rust
// main.rs — send_keyboard_event (idem souris, clipboard, chat, fichiers)
let bytes = msg.to_bytes()?;
webrtc.send_data(&bytes).await?;   // → base64 → VPS → pair, EN CLAIR
```

**Impact:** l'opérateur du VPS (ou quiconque compromet le VPS / se place sur nginx qui termine le TLS) voit **en clair** : chaque frappe clavier (donc les mots de passe tapés dans la session distante), le presse-papiers synchronisé, le chat, et **le contenu intégral des fichiers transférés**. Il peut aussi **injecter** des messages de contrôle (souris/clavier) vers le pair contrôlé. C'est la compromission totale de l'objectif « prise en main sécurisée ».

**Correctif recommandé:** chiffrer **tout** le canal de données au niveau applicatif (pas seulement la vidéo). Appliquer la clé E2E dans `Transport::send_data` / au niveau `RelayTransport`, une fois F2 résolu. Tant que ce n'est pas fait, le mot « E2E » ne doit pas figurer dans la comm produit.

---

## F2 — [CRITIQUE] Échange de clés E2E non authentifié (MITM par le relay)

**Fichier:** `client/src-tauri/src/main.rs` (`start_streaming` → `KeyExchangeInit`, `start_receiving`/`start_input_handler` → `KeyExchangeAccept`).

L'échange X25519 se fait en envoyant les clés publiques **en clair dans le canal relayé par le VPS**, sans aucune liaison à l'identité ni au mot de passe :

```
Hôte → (VPS) → Viewer : KeyExchangeInit { public_key }
Viewer → (VPS) → Hôte : KeyExchangeAccept { public_key }
```

Rien n'authentifie ces clés publiques. Un VPS malveillant/compromis effectue un **MITM ECDH classique** : il substitue ses propres clés publiques aux deux pairs, dérive **les deux** secrets partagés, et déchiffre/re-chiffre tout le flux de façon transparente. Le challenge-response mot de passe (`crypto.rs`) authentifie l'établissement de connexion **mais n'est pas lié aux clés ECDH** — il ne protège donc pas l'échange.

Résultat : même quand le chiffrement vidéo s'active, il **n'offre aucune protection contre l'adversaire (le relay) qu'il est censé contrer**.

**Correctif recommandé:** lier l'ECDH au secret partagé du mot de passe (ex. dériver une clé via `HKDF(shared_secret ‖ password_raw_hash)`, ou signer/authentifier les clés publiques via un HMAC dérivé du mot de passe). En l'absence de mot de passe, afficher et faire comparer une **empreinte de session (SAS)** aux deux utilisateurs (comme Signal/AnyDesk). Sans authentification du KEx, l'E2E est cosmétique.

---

## F3 — [HAUTE] Vidéo chiffrée « au mieux » seulement

**Fichier:** `client/src/streaming.rs`.

1. Le KEx est **non bloquant** (`start_streaming` : « premières frames potentiellement en clair »). Les frames émises avant réception de `KeyExchangeAccept` partent **en clair**.
2. Fallback silencieux en clair si le chiffrement échoue :

```rust
// streaming.rs — sender task
let payload = if let Some(ref key) = sender_key {
    match crypto.encrypt(key, &bytes) {
        Ok(enc) => { /* [0xE2][nonce][ct] */ }
        Err(e) => { stream_diag(...); bytes }   // ⚠️ envoi NON chiffré
    }
} else { bytes };                                // ⚠️ pas de clé → clair
```

3. Côté réception, sans clé, le paquet `0xE2` est « traité tel quel » (rétrocompat) → aucune garantie.
4. `start()` logue lui-même : *« E2E encryption au niveau applicatif non intégré dans le pipeline vidéo »*.

**Impact:** en mode relay, une part du flux écran transite en clair par le VPS ; le chiffrement n'est jamais garanti.

**Correctif:** rendre le chiffrement **obligatoire** — bloquer l'émission tant que la clé de session n'est pas dérivée et authentifiée (F2), supprimer le fallback en clair, refuser les paquets non chiffrés côté réception quand une clé est attendue.

---

## F4 — [MOYENNE] Régression entropie du Device ID

**Fichier:** `client/src/network.rs:1271` (`generate_device_id`).

```rust
let random_part: u32 = rand::thread_rng().gen();
format!("GHD-{:x}-{:x}", timestamp, random_part)   // 32 bits aléatoires + timestamp
```

L'audit v0.1.0 avait explicitement corrigé ceci vers **128 bits via `ring::SystemRandom`** (cf. `AUDIT_REPORT.md`). La version courante est **revenue** à timestamp + `u32` (32 bits, `rand::thread_rng`). ID plus devinable pour des connexions ciblées (mitigé par acceptation manuelle + mot de passe, mais c'est une régression).

**Correctif:** régénérer via `ring::rand::SystemRandom`, 128 bits (`GHD-{32 hex}`), comme spécifié dans l'audit initial.

---

## F5 — [MOYENNE] `KeyboardEvent::Type` contourne la whitelist

**Fichier:** `client/src/input_control.rs`.

`is_key_blocked` n'est vérifié que pour `Press`/`Release`. La variante `KeyboardEvent::Type { text }` appelle directement `enigo.text(&text)` **sans aucun filtrage**. Un pair (ou un VPS injectant du contrôle, cf. F1) peut saisir un texte arbitraire. Plus généralement, la whitelist de touches est de la sécurité fragile une fois la prise en main autorisée (RCE-by-design) : elle bloque Win+R mais pas la saisie de commandes dans un terminal déjà ouvert.

**Correctif:** appliquer la même politique de filtrage à `Type`, ou assumer que la whitelist est un garde-fou UX et non une frontière de sécurité (le vrai contrôle reste l'autorisation de connexion + mot de passe).

---

## F6 — [FAIBLE] Réception de fichiers sans garde-fou

**Fichier:** `client/src/file_transfer.rs::complete`.

Le path traversal est bien neutralisé (`file_name()`). Mais le fichier est écrit dans `Downloads/GhostHandDesk` **sans confirmation utilisateur et sans protection contre l'écrasement** (`std::fs::write` écrase). Un pair autorisé peut déposer/écraser des fichiers silencieusement.

**Correctif:** demander confirmation à la réception, suffixer en cas de collision (`nom (1).ext`), et notifier l'UI.

---

## F7 — [FAIBLE] `/stats` public si `STATS_TOKEN` vide

**Fichier:** `server/cmd/signaling/main.go`. Le `.env.example` documente correctement que `STATS_TOKEN` vide rend `/stats` public. L'endpoint ne renvoie qu'un compteur agrégé (pas la liste des Device IDs, bien), mais fuite le nombre d'utilisateurs et l'uptime. **Vérifier que `STATS_TOKEN` est défini sur le VPS de prod.**

---

## F8 — [INFO] Absence de fichiers `capabilities/` Tauri

Aucun fichier de capabilities (`client/src-tauri/capabilities/*.json`) n'est présent ; `gen/schemas/capabilities.json` = `{}`. `tauri_plugin_shell` est enregistré (`main.rs:1743`). À vérifier : que le plugin shell n'expose **aucune** permission `shell:allow-execute`/`allow-spawn` à la webview (sinon un XSS deviendrait une RCE). En l'état, l'absence de grant est le comportement **sûr** — à confirmer sur le build de release. La **CSP** (`tauri.conf.json`) est stricte et correcte.

---

## Points positifs (audits précédents bien tenus)

- Crypto primitives correctes : PBKDF2-SHA256 100k, AES-256-GCM, X25519, comparaisons constant-time (`ring::pbkdf2::verify`, `hmac::verify`).
- Serveur Go : limite clients (MaxClients), rate-limit fenêtre glissante + par IP (anti-rotation de port), TTL des pending connections, nettoyage anti-memory-leak, validation Device ID/SDP/ICE, tailles bornées, autorisation des paires relay (`GetRelayPartner`, anti-spoof `from`).
- Rejet auto-connexion, message 404 explicite, headers HTTP (`X-Content-Type-Options`, `X-Frame-Options`).
- Path traversal fichiers neutralisé, validation d'URL WebSocket (`ws://`/`wss://`), CSP Tauri stricte.
- Zéro `.unwrap()` dangereux en chemin critique (les `NonZeroU32::new(100_000).unwrap()` sont des constantes sûres).

---

## Corrections appliquées — v0.5.2 → v0.5.3

Toutes les failles ci-dessus ont été corrigées dans le code (lib + Tauri). Build : `cargo check` lib + Tauri OK, `go build ./...` OK, 52 tests unitaires + tests d'intégration/sécurité verts (le seul échec `test_complete_encoding_pipeline` est **préexistant** et lié à la capture/encodage vidéo, non à ces changements).

| # | Correctif |
|---|-----------|
| **F2** | `crypto::derive_session_key` (HKDF-SHA256) lie l'ECDH au secret d'authentification dérivé du mot de passe. Le secret est établi via le challenge-response (`SessionManager.auth_secret`, propagé à `AppState.e2e_auth_secret`) et injecté aux deux points de dérivation (hôte + viewer). Un MITM du relais sans le mot de passe ne peut plus dériver la clé. Test `test_derive_session_key_symmetry_and_binding`. |
| **F1** | Clé de session désormais **vive** (`SessionKeyHandle` partagé, relu à chaque trame — l'ancienne capture-par-valeur ne s'activait jamais). Tout le trafic sortant (souris, clavier, presse-papiers, chat, fichiers, contrôle écran) est scellé via `seal_control`. Côté hôte, `start_input_handler` **rejette tout input en clair** dès que la clé est active (anti-injection VPS). |
| **F3** | Le streamer **refuse d'émettre en clair** : sans clé de session dérivée, les trames sont écartées (plus de fenêtre en clair, plus de fallback silencieux). Réassemblage des fragments **avant** déchiffrement (l'ordre inverse cassait les grosses trames scellées). |
| **F4** | `generate_device_id` régénéré via `ring::SystemRandom`, 128 bits (`GHD-` + 32 hex). |
| **F5** | `KeyboardEvent::Type` borné (8 KiB) et filtré (rejet des octets de contrôle hors `\r\n\t`), avec log d'audit. |
| **F6** | Réception de fichiers : `unique_path` suffixe `nom (n).ext` — plus d'écrasement silencieux. |
| **F7** | Aucun changement de code (config VPS) — **action manuelle requise** : définir `STATS_TOKEN` sur le VPS. |
| **F8** | Observation seule — vérifier au build de release qu'aucun grant `shell:allow-*` n'atteint la webview. |

### Empreinte de session (SAS) — résidu F2 sans mot de passe fermé
Ajout d'une **empreinte de session (SAS)** : `crypto::session_fingerprint(session_key)` = SHA-256(contexte ‖ clé) tronqué à 64 bits, formaté `XXXX-XXXX-XXXX-XXXX`. Identique des deux côtés sans MITM, **divergente** si un attaquant s'intercale (il négocie deux clés distinctes). Exposée via la commande `get_session_fingerprint` et l'événement `ghosthand-session-secure` ; affichée dans un bandeau UI (App.vue) que les deux utilisateurs comparent hors-bande. Ceci authentifie la session **même sans mot de passe** (64 bits → grinding hors ligne infaisable). Test `test_session_fingerprint`.

### Résidus restants (mineurs)
- Le `Receiver` (viewer) reste **tolérant au clair** pendant le handshake (nécessaire pour `KeyExchangeInit`) ; un VPS pourrait injecter un message de contrôle **non-input** en clair au viewer (faux nom d'écran/chat). Faible risque ; l'input (direction dangereuse) est strictement protégé côté hôte, la vidéo est toujours scellée, et le SAS révèle une session non authentique.

## Priorités de remédiation

1. **F2 + F1 + F3 ensemble** (même correctif de fond) : rendre le chiffrement applicatif **obligatoire sur tout le canal** et **authentifier le KEx** via le mot de passe / SAS. C'est le sujet n°1 — sans lui, « E2E » est faux dès qu'on passe par le VPS.
2. **F4** : restaurer l'entropie 128 bits du Device ID.
3. **F5/F6** : filtrer `Type`, confirmer/anti-écraser les fichiers reçus.
4. **F7/F8** : vérifier `STATS_TOKEN` prod + absence de grant shell au build.
