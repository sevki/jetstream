# Guides

This section contains practical guides for working with JetStream's key features.

## Available Guides

### [Context](context.md)
Learn how to use JetStream's `Context` type to access connection metadata and peer information. This guide covers:
- Understanding remote addresses and peer information
- Accessing process credentials on Unix systems
- Implementing access control and audit logging
- Platform-specific features and best practices

### [Tracing](tracing.md)
Discover how to instrument your RPC services with distributed tracing. This guide covers:
- Enabling and configuring tracing in your services
- Automatic vs. custom instrumentation
- Structured logging and span management
- Integration with observability platforms
- Performance considerations and best practices

## Getting Started

If you're new to JetStream, we recommend starting with the [introduction](0intro.md) and then exploring these guides based on your needs:

1. **Need to identify callers?** → Start with [Context](context.md)
2. **Want to monitor and debug services?** → Start with [Tracing](tracing.md)

## Examples

All guides include practical examples. For complete, runnable examples, see the [examples directory](https://github.com/sevki/jetstream/tree/main/examples) in the repository:

- `echo.rs` - Basic RPC service
- `echo_with_tracing.rs` - RPC service with full tracing instrumentation
- `iroh_echo.rs` - Using iroh transport with context
