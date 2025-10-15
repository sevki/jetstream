# Tracing

JetStream provides built-in support for distributed tracing using the `tracing` crate. This allows you to instrument your RPC services and gain visibility into request flows, performance, and debugging information.

## Overview

Tracing in JetStream helps you:
- Track request execution across service boundaries
- Measure performance of individual RPC methods
- Debug issues with structured logging
- Monitor service behavior in production

## Enabling Tracing

Tracing is available through the `tracing` feature flag, which is enabled by default:

```toml
[dependencies]
jetstream = "9.4"  # tracing enabled by default
```

To explicitly control the feature:

```toml
[dependencies]
jetstream = { version = "9.4", default-features = false, features = ["tracing"] }
```

## Using Tracing in Services

### Automatic Instrumentation

The easiest way to add tracing is using the `#[service(tracing)]` attribute on your service trait:

```rust
use jetstream::prelude::*;
use jetstream_macros::service;

#[service(tracing)]  // Automatically instrument all methods
pub trait MyService {
    async fn process(&mut self, data: String) -> Result<String, Error>;
    async fn compute(&mut self, x: i32, y: i32) -> Result<i32, Error>;
}

struct MyServiceImpl;

impl MyService for MyServiceImpl {
    async fn process(&mut self, data: String) -> Result<String, Error> {
        // Method is automatically traced
        tracing::info!("Processing data of length {}", data.len());
        Ok(data.to_uppercase())
    }
    
    async fn compute(&mut self, x: i32, y: i32) -> Result<i32, Error> {
        // Method is automatically traced
        tracing::debug!("Computing {} + {}", x, y);
        Ok(x + y)
    }
}
```

### Custom Instrumentation

For more control, use the `#[instrument]` attribute on individual methods:

```rust
use jetstream::prelude::*;
use jetstream_macros::service;

#[service]
pub trait AdvancedService {
    /// Custom tracing with specific configuration
    #[instrument(
        name = "advanced_operation",
        skip(self, large_data),
        fields(
            data_len = large_data.len(),
            user_id = %user_id,
        ),
        level = "debug"
    )]
    async fn process_data(
        &mut self,
        user_id: String,
        large_data: Vec<u8>,
    ) -> Result<(), Error>;
    
    /// Simple instrumentation
    #[instrument(skip(self))]
    async fn simple_call(&mut self, value: i32) -> Result<i32, Error>;
}

struct AdvancedServiceImpl;

impl AdvancedService for AdvancedServiceImpl {
    async fn process_data(
        &mut self,
        user_id: String,
        large_data: Vec<u8>,
    ) -> Result<(), Error> {
        tracing::info!("Starting data processing");
        // Processing logic...
        tracing::info!("Data processing complete");
        Ok(())
    }
    
    async fn simple_call(&mut self, value: i32) -> Result<i32, Error> {
        Ok(value * 2)
    }
}
```

### Instrument Attribute Options

Common options for the `#[instrument]` attribute:

- `name = "span_name"`: Custom span name (default: function name)
- `skip(arg1, arg2)`: Skip specific arguments from the span
- `skip_all`: Skip all arguments
- `fields(key = value)`: Add custom fields to the span
- `level = "trace|debug|info|warn|error"`: Set the span level
- `err`: Automatically record errors
- `ret`: Record the return value

## Initializing the Tracing Subscriber

Before using tracing, initialize a subscriber in your `main` function:

### Basic Setup

```rust
#[tokio::main]
async fn main() {
    // Simple initialization with default settings
    tracing_subscriber::fmt::init();
    
    // Your application code...
}
```

### Advanced Configuration

```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_span_events(
            fmt::format::FmtSpan::ENTER | fmt::format::FmtSpan::EXIT
        )
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    // Your application code...
}
```

### Environment-based Configuration

Use the `RUST_LOG` environment variable to control log levels:

```bash
# Set global level
RUST_LOG=debug cargo run

# Set per-module levels
RUST_LOG=info,my_service=debug,jetstream=trace cargo run

# Complex filtering
RUST_LOG="warn,my_service::handler=debug" cargo run
```

## Complete Example

Here's a full example from the JetStream repository showing tracing in action:

```rust
use std::{net::SocketAddr, path::Path};
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::{client::ClientCodec, server::run, Framed};
use s2n_quic::{client::Connect, provider::tls, Client, Server};

/// Service with automatic tracing enabled
#[service(tracing)]
pub trait Echo {
    /// This method has custom tracing configuration
    #[instrument(
        name = "echo_ping",
        skip(self),
        fields(
            message_len = message.len(),
        ),
        level = "debug"
    )]
    async fn ping(&mut self, message: String) -> Result<String, Error>;
    
    /// This method uses default auto-instrumentation
    async fn echo(&mut self, text: String) -> Result<String, Error>;
}

struct EchoImpl;

impl Echo for EchoImpl {
    async fn ping(&mut self, message: String) -> Result<String, Error> {
        tracing::info!("Ping received: {}", message);
        Ok(format!("Pong: {}", message))
    }
    
    async fn echo(&mut self, text: String) -> Result<String, Error> {
        tracing::info!("Echo received: {}", text);
        Ok(text)
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber with detailed configuration
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_thread_ids(true)
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::ENTER
                | tracing_subscriber::fmt::format::FmtSpan::EXIT,
        )
        .init();
    
    tracing::info!("Starting echo service with tracing");
    
    // Server and client implementation...
}
```

## Tracing Levels

JetStream uses standard tracing levels:

| Level | Usage |
|-------|-------|
| `ERROR` | Critical errors that require immediate attention |
| `WARN` | Warning conditions that should be investigated |
| `INFO` | Informational messages about normal operation |
| `DEBUG` | Detailed information for debugging |
| `TRACE` | Very detailed trace information |

Example usage:

```rust
impl MyService for MyServiceImpl {
    async fn complex_operation(&mut self) -> Result<(), Error> {
        tracing::trace!("Entering complex operation");
        
        tracing::debug!("Step 1: Validating input");
        // validation logic...
        
        tracing::info!("Processing started");
        // main logic...
        
        if some_condition {
            tracing::warn!("Unusual condition detected");
        }
        
        if critical_error {
            tracing::error!("Critical error occurred");
            return Err(Error::from("Critical error"));
        }
        
        tracing::info!("Processing complete");
        Ok(())
    }
}
```

## Structured Logging

Tracing supports structured fields for better log analysis:

```rust
use jetstream::prelude::*;

async fn process_request(user_id: u64, request_id: String) -> Result<(), Error> {
    // Create a span with structured fields
    let span = tracing::info_span!(
        "process_request",
        user_id = %user_id,
        request_id = %request_id,
        status = tracing::field::Empty,
    );
    
    let _guard = span.enter();
    
    tracing::info!("Request started");
    
    // Update fields during execution
    span.record("status", "processing");
    
    // ... processing logic ...
    
    span.record("status", "complete");
    tracing::info!("Request completed");
    
    Ok(())
}
```

## Performance Considerations

1. **Use appropriate levels**: Use `debug` and `trace` for development, `info` and above for production
2. **Skip large data**: Use `skip()` to avoid logging large payloads
3. **Conditional compilation**: Tracing has minimal overhead when disabled, but you can also use compile-time feature gates
4. **Sampling**: For high-throughput services, consider implementing sampling

Example with conditional tracing:

```rust
#[service]
pub trait HighThroughputService {
    #[instrument(skip(self, large_payload), level = "debug")]
    async fn process(&mut self, id: u64, large_payload: Vec<u8>) -> Result<(), Error>;
}

impl HighThroughputService for MyImpl {
    async fn process(&mut self, id: u64, large_payload: Vec<u8>) -> Result<(), Error> {
        // Only log in debug builds
        #[cfg(debug_assertions)]
        tracing::debug!("Processing {} bytes", large_payload.len());
        
        // Production info (always compiled)
        if id % 1000 == 0 {  // Sample every 1000th request
            tracing::info!("Processed request {}", id);
        }
        
        Ok(())
    }
}
```

## Integration with Observability Tools

JetStream's tracing can be exported to various observability platforms:

### OpenTelemetry

```toml
[dependencies]
tracing-opentelemetry = "0.23"
opentelemetry = "0.22"
opentelemetry-jaeger = "0.21"
```

```rust
use opentelemetry::global;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("my-jetstream-service")
        .install_simple()
        .expect("Failed to install OpenTelemetry tracer");
    
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

### JSON Output

For structured log aggregation:

```rust
tracing_subscriber::fmt()
    .json()
    .with_max_level(tracing::Level::INFO)
    .init();
```

## Best Practices

1. **Use `#[service(tracing)]` for simple cases**: It provides good defaults for most services
2. **Add custom spans for business logic**: Use `#[instrument]` or manual spans for important operations
3. **Include context**: Add relevant fields like user IDs, request IDs, etc.
4. **Skip sensitive data**: Never log passwords, tokens, or other secrets
5. **Set appropriate levels**: Use debug/trace for verbose information, info for important events
6. **Test with tracing enabled**: Ensure your traces provide useful information
7. **Monitor overhead**: Profile your application to ensure tracing doesn't impact performance

## Troubleshooting

### No output when tracing

Make sure you've initialized a subscriber:
```rust
tracing_subscriber::fmt::init();
```

### Too much output

Adjust the log level:
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();
```

Or use environment variables:
```bash
RUST_LOG=info cargo run
```

### Missing spans

Ensure the `tracing` feature is enabled:
```toml
jetstream = { version = "9.4", features = ["tracing"] }
```

## See Also

- [Context documentation](context.md) - For accessing connection metadata
- [echo_with_tracing.rs example](https://github.com/sevki/jetstream/blob/main/examples/echo_with_tracing.rs) - Complete working example
- [tracing crate documentation](https://docs.rs/tracing) - Detailed tracing documentation
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber) - Subscriber configuration
