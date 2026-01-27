#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="$SCRIPT_DIR/../certs"

BROWSER="${1:-chrome}"

SPKI=$(openssl x509 -inform der -in "$CERTS_DIR/server.crt" -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl enc -base64)

echo "Got cert key $SPKI"

# Check if mTLS certs exist
if [ -f "$CERTS_DIR/client.p12" ]; then
    echo ""
    echo "=== mTLS Setup ==="
    echo ""
    echo "For Chrome/Chromium:"
    echo "  1. Import CA certificate (so browser trusts the server):"
    echo "     - Go to chrome://settings/certificates"
    echo "     - Click 'Authorities' tab"
    echo "     - Click 'Import' and select: $CERTS_DIR/ca.pem"
    echo "     - Check 'Trust this certificate for identifying websites'"
    echo ""
    echo "  2. Import client certificate (for mTLS authentication):"
    echo "     - Click 'Your Certificates' tab"
    echo "     - Click 'Import' and select: $CERTS_DIR/client.p12"
    echo "     - Password: changeit"
    echo ""
    echo "For Firefox:"
    echo "  1. Import CA certificate:"
    echo "     - Go to about:preferences#privacy"
    echo "     - Scroll to 'Certificates' and click 'View Certificates'"
    echo "     - Click 'Authorities' tab, then 'Import'"
    echo "     - Select: $CERTS_DIR/ca.pem"
    echo "     - Check 'Trust this CA to identify websites'"
    echo ""
    echo "  2. Import client certificate:"
    echo "     - Click 'Your Certificates' tab, then 'Import'"
    echo "     - Select: $CERTS_DIR/client.p12"
    echo "     - Password: changeit"
    echo ""
fi

launch_chrome() {
    echo "Opening Chrome/Chromium..."
    case $(uname) in
        Linux*)
            chromium --origin-to-force-quic-on=127.0.0.1:4433 --ignore-certificate-errors-spki-list=$SPKI https://localhost:4433/ 2>/dev/null &
            ;;
        Darwin*)
            open -a "Google Chrome" --args --origin-to-force-quic-on=127.0.0.1:4433 --ignore-certificate-errors-spki-list=$SPKI https://127.0.0.1:4433/
            ;;
    esac
    echo "Chrome launched. Navigate to https://localhost:4433/"
    echo ""
    echo "Logs: ~/Library/Application Support/Google/Chrome/chrome_debug.log (macOS)"
}

launch_firefox() {
    echo "Opening Firefox..."
    echo ""
    echo "Note: Firefox HTTP/3 support requires configuration:"
    echo "  1. Open about:config"
    echo "  2. Set network.http.http3.enable = true"
    echo "  3. Set network.http.http3.enable_qlog = true (optional, for debugging)"
    echo ""
    case $(uname) in
        Linux*)
            firefox https://127.0.0.1:4433/ 2>/dev/null &
            ;;
        Darwin*)
            open -a "Firefox" https://127.0.0.1:4433/
            ;;
    esac
    echo "Firefox launched. Navigate to https://127.0.0.1:4433/"
}

case "$BROWSER" in
    chrome|chromium)
        launch_chrome
        ;;
    firefox|ff)
        launch_firefox
        ;;
    *)
        echo "Usage: $0 [chrome|firefox]"
        echo ""
        echo "Browsers:"
        echo "  chrome, chromium  - Launch Chrome/Chromium with QUIC enabled"
        echo "  firefox, ff       - Launch Firefox"
        echo ""
        echo "Default: chrome"
        exit 1
        ;;
esac
