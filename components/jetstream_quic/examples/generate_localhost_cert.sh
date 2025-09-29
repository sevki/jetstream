#!/bin/bash

set -e

# Create a self-signed certificate for localhost that Chrome can use
# This certificate will work with the QUIC server on port 443

echo "Generating localhost certificate for Chrome QUIC testing"

# Generate private key
openssl genrsa -out localhost.key 2048

# Generate certificate signing request
openssl req -new -key localhost.key -out localhost.csr -subj "/C=US/ST=Test/L=Test/O=Test/CN=localhost"

# Generate self-signed certificate with SAN extension
cat > localhost.ext <<EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Generate certificate
openssl x509 -req -in localhost.csr -signkey localhost.key -out localhost.pem -days 365 -sha256 -extfile localhost.ext

# Convert to DER format for Chrome
openssl x509 -in localhost.pem -outform der -out localhost.crt

# Clean up temporary files
rm localhost.csr localhost.ext

echo "Certificate generation complete!"
echo "Files created:"
echo "  - localhost.key (private key in PEM format)"
echo "  - localhost.pem (certificate in PEM format)"
echo "  - localhost.crt (certificate in DER format for Chrome)"