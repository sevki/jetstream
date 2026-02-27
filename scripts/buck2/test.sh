#!/usr/bin/env bash
set -euo pipefail

cargo test -p jetstream --verbose --all-features
swift test
corepack enable
corepack prepare pnpm@latest --activate
pnpm install --frozen-lockfile
pnpm test
