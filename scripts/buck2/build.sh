#!/usr/bin/env bash
set -euo pipefail

cargo build -p jetstream --verbose
swift build
corepack enable
corepack prepare pnpm@latest --activate
pnpm install --frozen-lockfile
pnpm build
