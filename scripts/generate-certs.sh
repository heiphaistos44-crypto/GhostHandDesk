#!/bin/bash
# Script de génération de certificats TLS auto-signés pour GhostHandDesk
# Usage: ./generate-certs.sh [output_dir]

set -e

OUTPUT_DIR="${1:-../server/certs}"

echo ""
echo "================================================"
echo "   Génération de certificats TLS auto-signés"
echo "================================================"
echo ""

# Créer le dossier de sortie
if [ ! -d "$OUTPUT_DIR" ]; then
    mkdir -p "$OUTPUT_DIR"
    echo "✅ Dossier créé: $OUTPUT_DIR"
fi

CERT_FILE="$OUTPUT_DIR/cert.pem"
KEY_FILE="$OUTPUT_DIR/key.pem"

# Vérifier si OpenSSL est installé
if ! command -v openssl &> /dev/null; then
    echo "❌ ERREUR: OpenSSL non trouvé"
    echo ""
    echo "Installez OpenSSL:"
    echo "  - Ubuntu/Debian: sudo apt-get install openssl"
    echo "  - macOS: brew install openssl"
    echo "  - RHEL/CentOS: sudo yum install openssl"
    echo ""
    exit 1
fi

# Générer la clé privée ECDSA (P-256)
echo "🔑 Génération de la clé privée ECDSA..."
openssl ecparam -genkey -name prime256v1 -out "$KEY_FILE"
echo "✅ Clé privée générée: $KEY_FILE"

# Générer le certificat auto-signé (valide 365 jours)
echo "📜 Génération du certificat auto-signé..."
openssl req -new -x509 -key "$KEY_FILE" -out "$CERT_FILE" -days 365 \
    -subj "/C=FR/ST=Dev/L=Dev/O=GhostHandDesk/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

echo "✅ Certificat généré: $CERT_FILE"

echo ""
echo "================================================"
echo "✅ CERTIFICATS GÉNÉRÉS AVEC SUCCÈS"
echo "================================================"
echo ""
echo "Fichiers créés:"
echo "  - Certificat: $CERT_FILE"
echo "  - Clé privée: $KEY_FILE"
echo ""
echo "⚠️  AVERTISSEMENT:"
echo "  Ces certificats sont auto-signés et destinés"
echo "  AU DÉVELOPPEMENT UNIQUEMENT."
echo ""
echo "  En production, utilisez des certificats signés"
echo "  par une autorité reconnue (Let's Encrypt, etc.)"
echo ""
echo "Configuration serveur:"
echo "  export CERT_FILE=$CERT_FILE"
echo "  export KEY_FILE=$KEY_FILE"
echo "  export REQUIRE_TLS=true"
echo ""
echo "================================================"
