GhostHandDesk v0.2.0 — Remote Desktop via WebRTC
==================================================

FICHIERS
  ghosthanddesk-tauri.exe   Application de bureau à distance
  signaling-server.exe      Serveur de signalement (auto-hébergé)

UTILISATION RAPIDE (LAN / Internet via VPS)
  1. Lancez ghosthanddesk-tauri.exe sur les deux PCs
  2. Dans Paramètres, configurez l'URL du serveur signaling :
       - VPS public : wss://votre-vps.example.com/ws
       - LAN        : ws://192.168.x.x:9000/ws

UTILISATION SANS VPS (P2P auto-hébergé)
  PC A (hôte / serveur signaling) :
    1. Ouvrez le port 9000 (TCP) sur votre routeur / pare-feu
    2. Lancez le serveur : signaling-server.exe
    3. Lancez l'application : ghosthanddesk-tauri.exe
    4. Communiquez votre IP publique au PC distant

  PC B (client) :
    1. Dans Paramètres → URL signaling : ws://<IP_PUBLIQUE_PC_A>:9000/ws
    2. Entrez l'ID du PC A → Connecter

VARIABLES D'ENVIRONNEMENT (serveur)
  ALLOW_ALL_ORIGINS=true   Accepter les connexions depuis n'importe quel hôte
  GHD_SERVER_PORT=9000     Port d'écoute (défaut : 9000)
  CERT_FILE=cert.pem       Activer TLS (WSS)
  KEY_FILE=key.pem

SÉCURITÉ
  - Chiffrement E2E AES-256-GCM
  - NAT traversal via STUN (Google, Cloudflare, Mozilla)
  - PBKDF2-SHA256 (100 000 itérations) pour les mots de passe

Source : https://github.com/heiphaistos44-crypto/GhostHandDesk
Licence : MIT
