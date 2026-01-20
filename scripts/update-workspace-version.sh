#!/bin/bash
# Update all workspace versions to match the release version
# Usage: ./scripts/update-workspace-version.sh <version>
# Example: ./scripts/update-workspace-version.sh 10.1.0

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 10.1.0"
    exit 1
fi

VERSION="$1"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Updating workspace versions to $VERSION in $ROOT_DIR"

cd "$ROOT_DIR"

# Update root Cargo.toml - package version (first occurrence)
sed -i '0,/^version = "[0-9]*\.[0-9]*\.[0-9]*"/s//version = "'"$VERSION"'"/' Cargo.toml
echo "Updated root package version"

# Update workspace.package version
sed -i '/^\[workspace\.package\]/,/^\[/{s/^version = "[0-9]*\.[0-9]*\.[0-9]*"/version = "'"$VERSION"'"/}' Cargo.toml
echo "Updated workspace.package version"

# Update all workspace.dependencies jetstream crates with path
sed -i 's/\(jetstream[_a-z0-9]* = { path = "[^"]*", version = \)"[0-9]*\.[0-9]*\.[0-9]*"/\1"'"$VERSION"'"/g' Cargo.toml
echo "Updated workspace.dependencies versions"

# Update individual component Cargo.toml files
for component in components/*/Cargo.toml; do
    if [ -f "$component" ]; then
        # Only update if it has an explicit version (not workspace = true)
        if grep -q '^version = "[0-9]' "$component"; then
            sed -i 's/^version = "[0-9]*\.[0-9]*\.[0-9]*"/version = "'"$VERSION"'"/' "$component"
            echo "Updated $component"
        fi
    fi
done

# Update fuzz Cargo.toml if it has explicit version
if [ -f "fuzz/Cargo.toml" ] && grep -q '^version = "[0-9]' "fuzz/Cargo.toml"; then
    sed -i 's/^version = "[0-9]*\.[0-9]*\.[0-9]*"/version = "'"$VERSION"'"/' fuzz/Cargo.toml
    echo "Updated fuzz/Cargo.toml"
fi

# Update .release-please-manifest.json
if [ -f ".release-please-manifest.json" ]; then
    sed -i 's/"\.": "[0-9]*\.[0-9]*\.[0-9]*"/"\.": "'"$VERSION"'"/' .release-please-manifest.json
    echo "Updated .release-please-manifest.json"
fi

echo ""
echo "Done. Verifying with cargo check..."
cargo check --workspace 2>&1 | tail -5 || true
