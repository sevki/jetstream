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

- **TLS/mTLS**: Powered by [s2n-tls](https://github.com/aws/s2n-tls) for QUIC connections
- **Certificate Management**: Self-signed certificates in the repository are for **testing purposes only**
- **Transport Security**: Built on [s2n-quic](https://github.com/aws/s2n-quic) for secure, encrypted communication

### Best Practices for Users

When using JetStream in your applications:

1. **Certificate Management**
   - Never use the development certificates (`certs/` directory) in production
   - Always generate new certificates for production environments
   - Use certificates from trusted Certificate Authorities (CAs) when possible
   - Regularly rotate certificates and private keys

2. **Key Storage**
   - Never commit private keys to version control
   - Use secure key storage mechanisms (e.g., hardware security modules, key management services)
   - Restrict file permissions on private key files (e.g., `chmod 600`)

3. **Network Configuration**
   - Use mTLS for mutual authentication between clients and servers
   - Configure appropriate timeout values for connections
   - Implement rate limiting to prevent abuse

4. **Dependencies**
   - Keep JetStream and all dependencies up to date
   - Monitor security advisories for dependencies
   - Use `cargo audit` regularly to check for known vulnerabilities

5. **Input Validation**
   - Validate all input from untrusted sources
   - Be cautious when deserializing data from the network
   - Set appropriate limits on message sizes and connection counts

6. **Development vs Production**
   - JetStream is **not yet ready for production use**
   - Thoroughly test security configurations before deployment
   - Enable all security features and use secure defaults

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
- Rust Security Advisory Database (for critical issues)

## Additional Resources

- [s2n-quic Security](https://github.com/aws/s2n-quic/blob/main/docs/SECURITY.md)
- [s2n-tls Security](https://github.com/aws/s2n-tls/blob/main/SECURITY.md)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

## Questions?

If you have questions about this security policy or JetStream's security in general, please open a discussion in the [GitHub Discussions](https://github.com/sevki/jetstream/discussions) or contact s@sevki.io.

---

**Last Updated**: 2025-10-14
