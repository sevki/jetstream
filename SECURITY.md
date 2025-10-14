# Security Policy

## Supported Versions

JetStream is currently in active development and not yet ready for production use. We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 9.x.x   | :white_check_mark: |
| 8.x.x   | :white_check_mark: |
| < 8.0   | :x:                |

## Reporting a Vulnerability

We take the security of JetStream seriously. If you believe you have found a security vulnerability in JetStream, please report it to us as described below.

### Reporting Process

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to:
- **Security Contact**: s@sevki.io

Please include the following information in your report:
- Type of vulnerability
- Full paths of source file(s) related to the manifestation of the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability, including how an attacker might exploit it

### Response Timeline

- **Initial Response**: We will acknowledge receipt of your vulnerability report within 48 hours.
- **Status Update**: We will send you regular updates about our progress, at least every 7 days.
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days of the initial report.

### Disclosure Policy

- We follow a coordinated disclosure process
- We request that you do not publicly disclose the vulnerability until we have had a chance to address it
- Once a fix is available, we will:
  1. Release a security patch
  2. Publish a security advisory
  3. Credit you for the discovery (unless you prefer to remain anonymous)

## Security Considerations

### Cryptographic Components

JetStream uses several cryptographic and security-sensitive components:

#### Transport Layer Security

- **s2n-quic**: AWS's implementation of QUIC protocol with [s2n-tls](https://github.com/aws/s2n-tls) for TLS
- **rustls**: Modern TLS library used by Quinn and Iroh transports
  - Used in `jetstream_quic` component via [quinn](https://github.com/quinn-rs/quinn)
  - Used in `jetstream_iroh` component via [iroh-quinn](https://github.com/n0-computer/iroh)
  - Backed by [ring](https://github.com/briansmith/ring) for cryptographic primitives
- **quinn**: Pure Rust QUIC implementation with rustls integration

#### Cryptographic Libraries

- **ed25519-dalek**: EdDSA signatures over Curve25519
  - Used for identity and signing operations
  - Version pinned for security and compatibility
- **ring**: Cryptographic primitives library (via rustls)
- **WebSocket Security**: TLS support via [tungstenite](https://github.com/snapview/tungstenite-rs) in `jetstream_websocket`

#### Certificate Management

- **Certificate Management**: Self-signed certificates in the repository are for **testing purposes only**
- Supports both s2n-tls (with s2n-quic) and rustls (with Quinn/Iroh) certificate formats

### Security Guidelines

#### Important Warnings

- **Production Readiness**: JetStream is **not yet ready for production use**
- **Test Certificates**: Never use the development certificates (`certs/` directory) in production. These are for testing only.
- **Certificate Formats**: When configuring TLS, ensure you use the correct certificate format for your chosen transport (s2n-tls format for s2n-quic, rustls format for quinn/iroh)

#### Transport-Specific Considerations

When choosing and configuring a transport layer:

- **s2n-quic**: Requires s2n-tls compatible certificates. Refer to [s2n-tls security policy](https://github.com/aws/s2n-tls/blob/main/docs/USAGE-GUIDE.md) for cipher suite configuration.
- **quinn (jetstream_quic)**: Uses rustls certificates. Configure using `rustls::ServerConfig` and `rustls::ClientConfig`.
- **iroh (jetstream_iroh)**: Implements a peer-to-peer security model with built-in identity management.
- **WebSocket (jetstream_websocket)**: TLS configuration depends on your WebSocket server setup.

### Known Limitations

- JetStream is in early development and should not be used in production environments
- Security audits have not been performed
- The API and security model may change in future versions

## Security-Related Configuration

### TLS Configuration

When configuring TLS for JetStream:

```rust
// Use strong cipher suites and security policies
// Refer to s2n-tls security policies:
// https://github.com/aws/s2n-tls/blob/main/docs/USAGE-GUIDE.md
```

### Certificate Generation

For testing purposes only, you can use the provided scripts:
- `certs/generate_certs.sh` - Generate mTLS certificates for client/server
- `components/jetstream_quic/examples/generate_localhost_cert.sh` - Generate localhost certificates

**Warning**: These scripts generate self-signed certificates suitable only for development and testing.

## Security Advisories

Security advisories will be published in the following locations:
- [GitHub Security Advisories](https://github.com/sevki/jetstream/security/advisories)
- Repository [CHANGELOG.md](CHANGELOG.md)
- [Rust Security Advisory Database](https://rustsec.org/) (for critical issues)

## Additional Resources

### Security Documentation

- [s2n-quic Security](https://github.com/aws/s2n-quic/blob/main/docs/SECURITY.md)
- [s2n-tls Security](https://github.com/aws/s2n-tls/blob/main/SECURITY.md)
- [rustls Documentation](https://docs.rs/rustls/)
- [Quinn Security Considerations](https://github.com/quinn-rs/quinn#security)
- [Iroh Security](https://github.com/n0-computer/iroh)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

### Transport Security

JetStream supports multiple transport backends, each with different security implementations:

- **QUIC (s2n-quic)**: Uses s2n-tls for TLS 1.3
- **QUIC (quinn)**: Uses rustls for TLS 1.3  
- **Iroh**: Uses rustls with custom peer-to-peer security model
- **WebSocket**: Uses tungstenite with optional TLS

## Questions?

If you have questions about this security policy or JetStream's security in general, please open a discussion in the [GitHub Discussions](https://github.com/sevki/jetstream/discussions) or contact s@sevki.io.

---

**Last Updated**: 2025-10-14
