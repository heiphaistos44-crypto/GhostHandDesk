# Quick Start - Tests End-to-End

**Temps requis:** 10 minutes
**Prérequis:** Go, FFmpeg, Rust, Node.js

---

## 🚀 Démarrage Rapide

### 1️⃣ Vérifier les Prérequis (30 secondes)

```powershell
cd Documents/GhostHandDesk
.\scripts\check-prerequisites.ps1
```

**✅ Si tous les prérequis sont OK**, passez à l'étape 2.

**❌ Si des outils manquent**, installez-les :

```powershell
# Installer via Chocolatey (recommandé)
choco install golang ffmpeg openssl -y

# Redémarrer le terminal après installation
```

---

### 2️⃣ Lancer les Tests Automatiques (5 minutes)

```powershell
# Tests complets (unitaires + intégration + serveur)
.\scripts\run-e2e-tests.ps1 -FullSuite

# OU tests rapides (unitaires + intégration seulement)
.\scripts\run-e2e-tests.ps1 -QuickTest

# OU serveur uniquement (pour tests manuels)
.\scripts\run-e2e-tests.ps1 -ServerOnly
```

**Résultat attendu :**
```
========================================
  GhostHandDesk - Tests End-to-End
========================================

✅ Tous les prérequis critiques sont satisfaits !
✅ Compilation terminée
✅ Tests unitaires OK (18/18)
✅ Tests d'intégration OK (8/8)
✅ Serveur fonctionnel sur https://localhost:8443

Tests terminés avec succès !
```

---

### 3️⃣ Test Manuel de Connexion (5 minutes)

#### Terminal 1 : Serveur (si pas déjà lancé)

```powershell
cd server
go run cmd/signaling/main.go
```

**Attendez de voir :**
```
🚀 GhostHandDesk v0.1.0
[MAIN] Serveur de signalement démarré sur :8443
```

#### Terminal 2 : Client Host (Machine A)

```powershell
cd client
cargo tauri dev
```

**Attendez l'ouverture de l'interface Tauri.**

**📝 IMPORTANT : Notez le Device ID affiché** (ex: `GHD-abc123def456`)

#### Terminal 3 : Client Remote (Machine B)

```powershell
# Sur la même machine ou une autre
cd client
cargo tauri dev
```

**Dans l'interface qui s'ouvre :**
1. Entrez le Device ID de la Machine A
2. Laissez le mot de passe vide
3. Cliquez "Se connecter"

**✅ Validation :**
- Status passe à "Connecté"
- Canvas affiche l'écran de la Machine A
- Vous pouvez contrôler la souris et le clavier

---

## 🧪 Tests Manuels Rapides

### Test Streaming Vidéo

1. Sur Machine A, déplacez des fenêtres
2. Sur Machine B, vérifiez que vous voyez les mouvements en temps réel
3. Latence attendue : < 50ms (LAN)

### Test Contrôle Souris

1. Sur Machine B, déplacez la souris sur le canvas
2. Cliquez sur une icône du bureau de Machine A
3. Vérifiez que l'action s'exécute

### Test Contrôle Clavier

1. Sur Machine B, ouvrez Notepad (via souris)
2. Tapez "Test GhostHandDesk"
3. Vérifiez que le texte apparaît sur Machine A

---

## 📊 Indicateurs de Performance

**Pendant le streaming, vérifiez :**

| Indicateur | Valeur Attendue | Où Voir |
|------------|-----------------|---------|
| **FPS** | ≥ 25 | Interface RemoteViewer (en haut) |
| **Latence** | < 50ms (LAN) | Interface RemoteViewer |
| **CPU Host** | < 30% | Gestionnaire des tâches Windows |
| **CPU Remote** | < 20% | Gestionnaire des tâches Windows |
| **RAM** | < 300MB | Gestionnaire des tâches Windows |

**Commande PowerShell :**
```powershell
Get-Process GhostHandDesk | Select-Object CPU,WorkingSet
```

---

## 🐛 Dépannage Rapide

### Le serveur ne démarre pas

**Erreur : "Port 8443 already in use"**

```powershell
# Trouver le processus
netstat -ano | findstr :8443

# Tuer le processus (remplacer <PID> par le numéro affiché)
taskkill /PID <PID> /F
```

### Le client ne se compile pas

**Erreur : "cargo: command not found"**

```powershell
# Installer Rust
# Visitez https://rustup.rs et suivez les instructions

# Redémarrer le terminal après installation
```

**Erreur : "linker error" ou "vcpkg"**

```powershell
# Installer Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/
# Sélectionner "Desktop development with C++"
```

### La connexion échoue

**Vérifier :**

1. ✅ Le serveur tourne (Terminal 1 affiche les logs)
2. ✅ Les 2 clients sont lancés
3. ✅ Le Device ID est correctement copié (sans espaces)
4. ✅ Le pare-feu Windows n'bloque pas le port 8443

**Logs utiles :**

```powershell
# Logs serveur : Terminal 1 (stdout)
# Logs client backend : Terminal 2/3 (stdout)
# Logs client frontend : F12 dans l'interface → Console
```

### FFmpeg non détecté

**Si vous voyez "FFmpeg non disponible, fallback JPEG" :**

```powershell
# Installer FFmpeg
choco install ffmpeg -y

# Vérifier
ffmpeg -version

# Redémarrer le terminal
# Recompiler le client
cd client
cargo clean
cargo build --release --features ffmpeg
```

---

## 📝 Rapport de Tests

Après vos tests, remplissez le rapport :

```powershell
# Copier le template
cp E2E_TEST_RESULTS_TEMPLATE.md E2E_TEST_RESULTS.md

# Éditer avec vos résultats
notepad E2E_TEST_RESULTS.md
```

---

## 📚 Documentation Complète

Pour des tests plus approfondis, consultez :

- **Guide E2E complet :** `E2E_TESTING_GUIDE.md`
- **Guide de lancement :** `LAUNCH.md`
- **Rapport de session :** `SESSION_REPORT.md`
- **README principal :** `README.md`

---

## ✅ Checklist de Validation

Avant de déclarer les tests réussis :

- [ ] ✅ Script `check-prerequisites.ps1` passe
- [ ] ✅ Script `run-e2e-tests.ps1 -FullSuite` passe
- [ ] ✅ Serveur démarre sans erreur
- [ ] ✅ 2 clients se lancent
- [ ] ✅ Connexion WebRTC établie
- [ ] ✅ Streaming vidéo fonctionnel
- [ ] ✅ Contrôle souris fonctionnel
- [ ] ✅ Contrôle clavier fonctionnel
- [ ] ✅ FPS ≥ 25
- [ ] ✅ Latence < 50ms
- [ ] ✅ Déconnexion propre

**Si toutes les cases sont cochées : 🎉 Tests E2E réussis !**

---

## 🆘 Support

**En cas de problème persistant :**

1. Consultez `E2E_TESTING_GUIDE.md` (section Troubleshooting)
2. Vérifiez les logs (serveur + client)
3. Relancez les tests unitaires : `cd client && cargo test`
4. Créez une issue GitHub (à venir)

**Logs de debug :**

```powershell
# Serveur avec logs détaillés
$env:LOG_LEVEL="debug"
go run cmd/signaling/main.go

# Client avec traces
$env:RUST_LOG="debug"
cargo tauri dev
```

---

**🚀 Prêt à tester ! Bonne chance !**
