#!/bin/bash
# Script de déploiement VPS — GhostHandDesk
# Usage: bash deploy-vps.sh
set -e

REPO="git@github.com:heiphaistos44-crypto/GhostHandDesk.git"
DEPLOY_DIR="/opt/ghosthanddesk"
PM2_NAME="ghosthanddesk"
PORT=9000
DOMAIN="ghd.heiphaistos.org"

echo "[INFO] === Déploiement GhostHandDesk sur $DOMAIN ==="

# ── 1. Vérifier Go ──────────────────────────────────────────────────────────
if ! command -v go &>/dev/null; then
    echo "[INFO] Go non trouvé — installation..."
    apt-get install -y golang-go
fi
echo "[INFO] Go: $(go version)"

# ── 2. Clone ou pull ─────────────────────────────────────────────────────────
if [ -d "$DEPLOY_DIR/.git" ]; then
    echo "[INFO] Pull dernière version..."
    git -C "$DEPLOY_DIR" pull origin main
else
    echo "[INFO] Clone du repo..."
    GIT_SSH_COMMAND='ssh -i ~/.ssh/github_deploy' \
        git clone "$REPO" "$DEPLOY_DIR"
fi

# ── 3. Build serveur Go ───────────────────────────────────────────────────────
echo "[INFO] Build du serveur Go..."
cd "$DEPLOY_DIR/server"
go build -ldflags="-s -w" -o signaling-server ./cmd/signaling/
echo "[INFO] Build OK → $DEPLOY_DIR/server/signaling-server"

# ── 4. PM2 ───────────────────────────────────────────────────────────────────
if pm2 describe "$PM2_NAME" &>/dev/null; then
    echo "[INFO] PM2 : redémarrage de $PM2_NAME..."
    pm2 restart "$PM2_NAME"
else
    echo "[INFO] PM2 : démarrage de $PM2_NAME via ecosystem..."
    pm2 start "$DEPLOY_DIR/ecosystem.config.js"
fi
pm2 save

# ── 5. Nginx ─────────────────────────────────────────────────────────────────
echo "[INFO] Configuration nginx..."
cp "$DEPLOY_DIR/deploy/nginx-ghd.conf" "/etc/nginx/sites-available/$DOMAIN"
ln -sf "/etc/nginx/sites-available/$DOMAIN" "/etc/nginx/sites-enabled/$DOMAIN"
nginx -t && systemctl reload nginx
echo "[INFO] Nginx OK"

# ── 6. SSL Let's Encrypt ─────────────────────────────────────────────────────
if [ ! -d "/etc/letsencrypt/live/$DOMAIN" ]; then
    echo "[INFO] Obtention certificat SSL pour $DOMAIN..."
    certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos -m admin@heiphaistos.org
else
    echo "[INFO] Certificat SSL déjà présent ✓"
fi

echo ""
echo "[OK] ✅ GhostHandDesk déployé sur https://$DOMAIN"
echo "[OK]    WebSocket signaling : wss://$DOMAIN/ws"
echo "[OK]    PM2 status :"
pm2 status "$PM2_NAME"
