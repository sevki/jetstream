#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

SPKI=$(openssl x509 -inform der -in localhost.crt -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl enc -base64)

echo "Got cert key $SPKI"

# Check if mTLS certs exist
if [ -f "client.p12" ]; then
    echo ""
    echo "=== mTLS Setup ==="
    echo "To use mTLS with Chrome, you need to import certificates:"
    echo ""
    echo "1. Import CA certificate (so Chrome trusts the server):"
    echo "   - Go to chrome://settings/certificates"
    echo "   - Click 'Authorities' tab"
    echo "   - Click 'Import' and select: $SCRIPT_DIR/ca.pem"
    echo "   - Check 'Trust this certificate for identifying websites'"
    echo ""
    echo "2. Import client certificate (for mTLS authentication):"
    echo "   - Click 'Your Certificates' tab"
    echo "   - Click 'Import' and select: $SCRIPT_DIR/client.p12"
    echo "   - Password: changeit"
    echo ""
    echo "When you connect, Chrome will prompt you to select a certificate."
    echo ""
fi

echo "Opening Chrome/Chromium..."

case $(uname) in
    Linux*)
        chromium --origin-to-force-quic-on=127.0.0.1:4433 --ignore-certificate-errors-spki-list=$SPKI https://localhost:4433/ 2>/dev/null &
        ;;
    Darwin*)
        open -a "Google Chrome" --args --origin-to-force-quic-on=127.0.0.1:4433 --ignore-certificate-errors-spki-list=$SPKI https://localhost:4433/
        ;;
esac

echo "Chrome launched. Navigate to https://localhost:4433/"

## Logs are stored to ~/Library/Application Support/Google/Chrome/chrome_debug.log
