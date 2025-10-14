# JetStream Copilot Instructions

## Project Overview

JetStream is a high-performance RPC framework for Rust, originally built on top of s2n-quic and the Plan 9 (9P) protocol. It provides tools for building distributed systems with features like bidirectional streaming, 0-RTT connection establishment, mTLS, and efficient binary encoding.

**Key Features:**
- Bidirectional streaming support
- 0-RTT connection establishment
- Mutual TLS (mTLS) authentication
- Custom binary wire format for efficient serialization
- Multi-transport support (QUIC, WebSocket, Iroh)
- Plan 9 filesystem protocol (9P) implementation

**Project Status:** Early development stage - not ready for production use

## Repository Structure

```
jetstream/
├── src/                          # Main library source
│   └── lib.rs                    # Main entry point with feature-gated modules
├── components/                   # Workspace components (sub-crates)
│   ├── jetstream_9p/            # 9P protocol implementation
│   ├── jetstream_iroh/          # Iroh transport
│   ├── jetstream_macros/        # Procedural macros (service, WireFormat)
│   ├── jetstream_quic/          # QUIC transport
│   ├── jetstream_rpc/           # RPC framework core
│   ├── jetstream_ufs/           # Unix filesystem implementation
│   ├── jetstream_websocket/     # WebSocket transport
│   └── jetstream_wireformat/    # Wire format serialization
├── examples/                     # Example applications
│   ├── echo.rs                  # Basic echo service example
│   └── iroh_echo.rs             # Iroh-based echo example
├── tests/                        # Integration tests
├── benches/                      # Benchmarks
├── fuzz/                         # Fuzzing tests
├── docs/                         # Documentation (mdBook)
└── certs/                        # TLS certificates for examples
```

## Coding Standards

### Rust Style Guidelines

1. **Code Formatting:**
   - Use `rustfmt` with the project's configuration (`rustfmt.toml`)
   - Max line width: 80 characters
   - Group imports by: Std → External → Crate
   - Use `imports_granularity = "Crate"` and `imports_layout = "Mixed"`

2. **Naming Conventions:**
   - Types: `PascalCase` (e.g., `EchoChannel`, `ClientCodec`)
   - Functions/methods: `snake_case` (e.g., `byte_size`, `encode_wire_format`)
   - Constants: `SCREAMING_SNAKE_CASE` (e.g., `CA_CERT_PEM`)
   - Modules: `snake_case` (e.g., `jetstream_rpc`)

3. **Documentation:**
   - Use `///` for public API documentation
   - Include module-level documentation with `//!`
   - Document all public traits, structs, and functions
   - Provide examples in documentation where helpful

4. **Error Handling:**
   - Use `Result<T, Error>` for fallible operations
   - Prefer custom error types from `jetstream_rpc::Error`
   - Use `?` operator for error propagation

5. **Async Code:**
   - Use `async-trait` for async traits
   - Prefer `async fn` for async methods
   - Use `tokio` for async runtime features

### Macros

The project provides custom procedural macros:

1. **`#[service]`** - Generates RPC service implementations
   ```rust
   #[service]
   pub trait Echo {
       async fn ping(&mut self) -> Result<(), Error>;
   }
   ```

2. **`#[derive(JetStreamWireFormat)]`** - Auto-derives wire format serialization
   - Use `#[jetstream(skip)]` attribute to skip fields during serialization

### Feature Flags

The project uses Cargo features for optional functionality:
- `9p` - Enable 9P protocol support
- `quic` - Enable QUIC transport
- `websocket` - Enable WebSocket transport
- `iroh` - Enable Iroh transport
- `wasm` - Enable WASM support
- `tokio` - Enable Tokio features
- `all` - Enable all transports (9p, iroh, quic)

Always consider feature-gating code appropriately using `#[cfg(feature = "...")]`

## Common Patterns

### Creating an RPC Service

1. Define the service trait with `#[service]` macro
2. Implement the trait for your service struct
3. Use the generated `Channel` type for client communication

### Wire Format Implementation

For custom types that need serialization:
```rust
#[derive(JetStreamWireFormat)]
struct MyType {
    field1: u32,
    #[jetstream(skip)]  // Skip this field
    field2: String,
}
```

### Testing

1. **Unit Tests:** Place in the same file using `#[cfg(test)]` mod tests
2. **Integration Tests:** Place in `tests/` directory
3. **Snapshot Tests:** Use `insta` for snapshot testing (see `jetstream_macros`)
4. **Benchmarks:** Place in `benches/` directory using `criterion`

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for a specific package
cargo test -p jetstream_rpc

# Run tests with all features
cargo test --all-features

# Run benchmarks
cargo bench
```

### Test Organization

- **Unit tests:** In-module tests for individual functions
- **Integration tests:** Cross-component tests in `tests/`
- **Example tests:** Tests that examples compile and run
- **Fuzzing:** Fuzz tests in `fuzz/` directory

### CI/CD

The project uses GitHub Actions for:
- Multi-platform testing (Linux, macOS, Windows)
- WASM target compilation
- Clippy linting
- Benchmarking on pull requests
- Security scanning (Scorecard)
- Documentation building (mdBook)

## Building and Development

### Build Commands

```bash
# Build with default features
cargo build

# Build with all features
cargo build --all-features

# Build specific component
cargo build -p jetstream_rpc

# Build for WASM
cargo build -p jetstream_wireformat --target wasm32-unknown-unknown
```

### Running Examples

```bash
# Run echo example
cargo run --example echo --features quic

# Run iroh echo example
cargo run --example iroh_echo --features iroh
```

## Common Issues and Solutions

1. **Feature Flag Confusion:**
   - Default features are minimal
   - Use `--all-features` for full functionality
   - Check `Cargo.toml` for available features

2. **Async Trait Issues:**
   - Always use `async-trait` crate for async traits
   - Generated code may require `#[async_trait]` attribute

## Related Resources

- [JetStream Book](https://sevki.github.io/jetstream) - Full documentation
- [API Documentation](https://jetstream.rs) - API reference
- [Repository](https://github.com/sevki/jetstream) - Source code
- Original inspiration: [CrosVM project](https://chromium.googlesource.com/chromiumos/platform/crosvm/)

## Contribution Guidelines

When contributing:
1. Follow the coding standards above
2. Add tests for new functionality
3. Update documentation for public APIs
4. Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting
5. Keep changes focused and minimal
6. Ensure CI passes on all platforms
7. Use [Conventional Commits](https://www.conventionalcommits.org/) format for commit messages - this project uses [release-please](https://github.com/googleapis/release-please) for automated releases
