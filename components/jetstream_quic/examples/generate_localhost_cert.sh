#!/bin/bash

set -e

# Create certificates for QUIC testing with optional mTLS
# Generates: CA, server cert, and client cert

echo "=== Generating certificates for QUIC/mTLS testing ==="

# --- 1. Generate CA ---
echo ""
echo "1. Generating CA certificate..."
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 3650 -key ca.key -out ca.pem \
    -subj "/C=US/ST=Test/L=Test/O=JetStream Test CA/CN=JetStream Test CA"

# --- 2. Generate Server Certificate ---
echo ""
echo "2. Generating server certificate (localhost)..."
openssl genrsa -out localhost.key 2048
openssl req -new -key localhost.key -out localhost.csr \
    -subj "/C=US/ST=Test/L=Test/O=JetStream/CN=localhost"

cat > localhost.ext <<EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

openssl x509 -req -in localhost.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
    -out localhost.pem -days 365 -sha256 -extfile localhost.ext

# Convert to DER format for Chrome
openssl x509 -in localhost.pem -outform der -out localhost.crt

rm localhost.csr localhost.ext

# --- 3. Generate Client Certificate ---
echo ""
echo "3. Generating client certificate..."
openssl genrsa -out client.key 2048
openssl req -new -key client.key -out client.csr \
    -subj "/C=US/ST=Test/L=Test/O=JetStream Client/CN=test-client"

cat > client.ext <<EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
subjectAltName = @alt_names

[alt_names]
email.1 = test@example.com
URI.1 = spiffe://jetstream.rs/client/test-client
EOF

openssl x509 -req -in client.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
    -out client.pem -days 365 -sha256 -extfile client.ext

# Create PKCS12 bundle for browsers
openssl pkcs12 -export -out client.p12 -inkey client.key -in client.pem \
    -certfile ca.pem -passout pass:changeit

rm client.csr client.ext

# --- Summary ---
echo ""
echo "=== Certificate generation complete! ==="
echo ""
echo "CA files:"
echo "  - ca.key     (CA private key)"
echo "  - ca.pem     (CA certificate - import to browser as trusted)"
echo ""
echo "Server files:"
echo "  - localhost.key  (server private key)"
echo "  - localhost.pem  (server certificate)"
echo "  - localhost.crt  (server cert in DER format)"
echo ""
echo "Client files (for mTLS testing):"
echo "  - client.key     (client private key)"
echo "  - client.pem     (client certificate)"
echo "  - client.p12     (PKCS12 bundle for browser import, password: changeit)"
echo ""
echo "To test mTLS with curl:"
echo "  curl --cert client.pem --key client.key --cacert ca.pem https://localhost:4433/"
echo ""
echo "To import client cert in Chrome:"
echo "  1. Go to chrome://settings/certificates"
echo "  2. Import ca.pem under 'Authorities'"
echo "  3. Import client.p12 under 'Your Certificates' (password: changeit)"
