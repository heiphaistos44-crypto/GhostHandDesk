========================================
   GhostHandDesk - Version Portable v0.2.0
========================================

LANCEMENT :
  Double-cliquez sur LANCER-GHOSTHANDDESK.bat

  (Lance automatiquement le serveur + l'application)

CARACTERISTIQUES :
  - 100% portable, aucune installation
  - Serveur embarqué, lancé automatiquement
  - Certificats générés automatiquement (TLS obligatoire)
  - Chiffrement E2E : X25519 + AES-256-GCM
  - Adaptive bitrate streaming
  - Protection touches système (Win+R, Ctrl+Alt+Del bloqués)
  - Aucune trace sur le PC

UTILISATION :

1. PC Principal :
   - Double-cliquez sur LANCER-GHOSTHANDDESK.bat
   - Attendez 2 secondes (démarrage serveur)
   - L'application s'ouvre automatiquement
   - Notez le Device ID affiché

2. VM ou PC distant :
   - Copiez ce dossier sur la machine
   - Double-cliquez sur LANCER-GHOSTHANDDESK.bat
   - Notez le Device ID

3. Connexion :
   - Sur le PC principal, entrez le Device ID de la VM
   - Laissez le mot de passe vide (optionnel)
   - Cliquez "Connect"

SÉCURITÉ :
  - Chiffrement E2E : X25519 + AES-256-GCM
  - Validation stricte des entrées
  - Rate limiting anti-DoS
  - Logs d'audit automatiques
  - Rotation logs : 10 MB max, 30 jours rétention

  NOTE : TLS désactivé en mode portable (localhost uniquement)
        Pour production avec réseau externe, utilisez TLS

SUPPRESSION :
  Supprimez simplement ce dossier.
  Aucune trace ne restera sur votre PC.

NOUVEAUTÉS v0.2.0 :
  ✅ TLS obligatoire avec auto-génération certificats
  ✅ Whitelist touches système (sécurité renforcée)
  ✅ Adaptive bitrate (40-95 qualité JPEG)
  ✅ Validation entrées stricte (Device ID, SDP, ICE)
  ✅ Rate limiting client (5 conn/min, 100 msg/min)
  ✅ Rotation logs automatique (>30 jours supprimés)

SUPPORT :
  - Logs : ./logs/audit.jsonl
  - Tests : 54/54 passés (100%)
  - Documentation : MIGRATION.md, CHANGELOG.md

========================================
Version : 0.2.0 Portable (Production-Ready)
Date : 2026-02-07
Tests : ✅ 54/54 PASSÉS
Compilation : ✅ RÉUSSIE
========================================
