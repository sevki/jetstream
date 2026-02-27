#!/bin/bash

# Generate certificates for QUIC/mTLS testing
# Outputs: ca.pem, ca.key, server.pem, server.key, client.pem, client.key, client.p12, server.crt

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Generating certificates for QUIC/mTLS testing ==="

# --- 1. Generate CA ---
echo ""
echo "1. Generating CA certificate..."
openssl req -nodes -new -x509 \
    -keyout ca.key \
    -out ca.pem \
    -days 3650 \
    -config config/ca.cnf

# --- 2. Generate Server Certificate ---
echo ""
echo "2. Generating server certificate (localhost)..."
openssl req -new -nodes \
    -newkey ec -pkeyopt ec_paramgen_curve:secp384r1 \
    -keyout server.key \
    -out server.csr \
    -config config/server.cnf

openssl x509 -req \
    -in server.csr \
    -CA ca.pem \
    -CAkey ca.key \
    -CAcreateserial \
    -out server.pem \
    -days 3650 \
    -sha256 \
    -extensions req_ext \
    -extfile config/server.cnf

# Convert to DER format for Chrome
openssl x509 -in server.pem -outform der -out server.crt

# --- 3. Generate Client Certificate ---
echo ""
echo "3. Generating client certificate..."
openssl req -new -nodes \
    -newkey ec -pkeyopt ec_paramgen_curve:secp384r1 \
    -keyout client.key \
    -out client.csr \
    -config config/client.cnf

openssl x509 -req \
    -in client.csr \
    -CA ca.pem \
    -CAkey ca.key \
    -CAcreateserial \
    -out client.pem \
    -days 3650 \
    -sha256 \
    -extensions req_ext \
    -extfile config/client.cnf

# Create PKCS12 bundle for browsers
openssl pkcs12 -export \
    -out client.p12 \
    -inkey client.key \
    -in client.pem \
    -certfile ca.pem \
    -passout pass:changeit

# --- 3b. Generate Second Client Certificate (for rejection testing) ---
echo ""
echo "3b. Generating second client certificate (unauthorized)..."
openssl req -new -nodes \
    -newkey ec -pkeyopt ec_paramgen_curve:secp384r1 \
    -keyout client2.key \
    -out client2.csr \
    -config config/client2.cnf

openssl x509 -req \
    -in client2.csr \
    -CA ca.pem \
    -CAkey ca.key \
    -CAcreateserial \
    -out client2.pem \
    -days 3650 \
    -sha256 \
    -extensions req_ext \
    -extfile config/client2.cnf

# --- 4. Verify certificates ---
echo ""
echo "4. Verifying certificates..."
openssl verify -CAfile ca.pem server.pem
openssl verify -CAfile ca.pem client.pem
openssl verify -CAfile ca.pem client2.pem

# --- 5. Inject SPKI hash into .playwright-mcp.json ---
echo ""
echo "5. Updating .playwright-mcp.json with SPKI hash..."
SPKI=$(openssl x509 -inform der -in server.crt -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl enc -base64)
echo "  SPKI hash: $SPKI"
PLAYWRIGHT_CONFIG="$SCRIPT_DIR/../.playwright-mcp.json"
if [ -f "$PLAYWRIGHT_CONFIG" ] && command -v sed >/dev/null 2>&1; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s|--ignore-certificate-errors-spki-list=.*\"|--ignore-certificate-errors-spki-list=$SPKI\"|" "$PLAYWRIGHT_CONFIG"
    else
        sed -i "s|--ignore-certificate-errors-spki-list=.*\"|--ignore-certificate-errors-spki-list=$SPKI\"|" "$PLAYWRIGHT_CONFIG"
    fi
    echo "  Updated $PLAYWRIGHT_CONFIG"
else
    echo "  Warning: $PLAYWRIGHT_CONFIG not found or sed not available, skipping"
fi

# --- 6. Clean up ---
echo ""
echo "6. Cleaning up temporary files..."
rm -f server.csr client.csr client2.csr ca.srl

# --- Summary ---
echo ""
echo "=== Certificate generation complete! ==="
echo ""
echo "CA files:"
echo "  - ca.key     (CA private key)"
echo "  - ca.pem     (CA certificate - import to browser as trusted)"
echo ""
echo "Server files:"
echo "  - server.key  (server private key)"
echo "  - server.pem  (server certificate)"
echo "  - server.crt  (server cert in DER format for Chrome)"
echo ""
echo "Client files (for mTLS testing):"
echo "  - client.key  (client private key)"
echo "  - client.pem  (client certificate)"
echo "  - client.p12  (PKCS12 bundle for browser import, password: changeit)"
echo ""
echo "To test mTLS with curl:"
echo "  curl --cert certs/client.pem --key certs/client.key --cacert certs/ca.pem https://localhost:4433/"
echo ""
echo "To import client cert in Chrome:"
echo "  1. Go to chrome://settings/certificates"
echo "  2. Import ca.pem under 'Authorities'"
echo "  3. Import client.p12 under 'Your Certificates' (password: changeit)"
