# Security Advisories Status

## Overview
This document tracks the status of security advisories (RustSec) for the JetStream project.

## Current Status (as of 2025-10-12)

### Fixed in Our Codebase
1. **RUSTSEC-2024-0436** (paste - unmaintained)
   - **Status**: Partially fixed in jetstream_wireformat
   - **Action Taken**: Migrated from `paste` (v1.0.15) to `pastey` (v0.1.1) in components/jetstream_wireformat
   - **Remaining**: Still present in transitive dependencies through `iroh` → `stun-rs` and `netlink-packet-utils`

### Transitive Dependency Issues (from iroh)
The following advisories are present in third-party dependencies and cannot be directly fixed without updates from upstream:

2. **RUSTSEC-2023-0089** (atomic-polyfill - unmaintained)
   - **Severity**: Warning (unmaintained, not a CVE)
   - **Source**: `iroh` → `iroh-metrics` → `postcard` → `heapless` → `atomic-polyfill`
   - **Recommended Alternative**: portable-atomic
   - **Status**: Waiting for upstream fix in the `iroh` ecosystem

3. **RUSTSEC-2024-0384** (instant - unmaintained)
   - **Severity**: Warning (unmaintained, not a CVE)
   - **Source**: `iroh` (direct dependency)
   - **Recommended Alternative**: web-time
   - **Status**: Waiting for upstream fix in `iroh`

## Notes
- All three advisories are "unmaintained" warnings, not critical security vulnerabilities with CVEs
- We have updated `iroh` from v0.92.0 to v0.93.1 (latest version as of 2025-10-12)
- The remaining issues require updates from the `iroh` project and its dependencies

## Mitigation
These are maintenance warnings rather than active security exploits. The crates themselves don't have known security vulnerabilities, but they are no longer actively maintained. The risk is low for current usage, but we should monitor for updates from the upstream `iroh` project.

## Action Items
- [ ] Monitor `iroh` releases for updates that address these dependencies
- [ ] Consider reporting these issues to the `iroh` project if not already tracked
- [ ] Periodically re-run `cargo audit` to check for new advisories or fixes
