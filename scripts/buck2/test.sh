#!/usr/bin/env bash
set -euo pipefail

cargo test -p jetstream --verbose --all-features
