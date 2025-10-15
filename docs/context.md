# Context

The `Context` type in JetStream provides access to connection metadata and peer information for RPC calls. It allows services to determine who is making requests and from where, enabling security, auditing, and routing decisions.

## Overview

The `Context` struct contains two main pieces of information:
- `remote`: The remote address of the connection
- `peer`: Information about the peer (process credentials on Unix, node ID for iroh)

## Key Components

### Context Structure

```rust
pub struct Context {
    remote: Option<RemoteAddr>,
    peer: Option<Peer>,
}
```

### Remote Address Types

The `RemoteAddr` enum represents different types of remote addresses:

```rust
pub enum RemoteAddr {
    UnixAddr(PathBuf),      // Unix domain socket path
    NodeAddr(NodeAddr),      // Iroh node address
    IpAddr(IpAddr),         // IP address (for QUIC/TCP)
}
```

### Peer Types

The `Peer` enum provides information about the connected peer:

```rust
pub enum Peer {
    Unix(Unix),        // Unix credentials (uid, gid, pid)
    NodeId(NodeId),    // Iroh node identifier
}
```

## Usage

### Obtaining Context from a Connection

Any framed connection that implements the `Contextual` trait can provide context:

```rust
use jetstream::prelude::*;

// For a Unix domain socket connection
let framed: Framed<UnixStream, _> = /* ... */;
let ctx = framed.context();

// For a QUIC connection
let framed: Framed<BidirectionalStream, _> = /* ... */;
let ctx = framed.context();
```

### Supported Transports

JetStream provides `Contextual` implementations for:
- **Unix Domain Sockets** (Linux/macOS): Provides path and process credentials
- **QUIC streams** (via s2n-quic): Provides IP address
- **TCP streams** (via turmoil): Provides IP address
- **Iroh connections**: Provides node address and ID

### Accessing Context Information

#### Remote Address

```rust
use jetstream::prelude::*;

fn check_remote_address(ctx: &Context) {
    if let Some(remote) = &ctx.remote {
        match remote {
            RemoteAddr::UnixAddr(path) => {
                println!("Connection from Unix socket: {:?}", path);
            }
            RemoteAddr::IpAddr(ip) => {
                println!("Connection from IP: {}", ip);
            }
            RemoteAddr::NodeAddr(node_addr) => {
                println!("Connection from Iroh node: {:?}", node_addr);
            }
        }
    }
}
```

#### Peer Credentials (Unix)

On Linux and macOS, you can access process credentials for Unix domain socket connections:

```rust
use jetstream::prelude::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn check_peer_credentials(ctx: &Context) {
    if let Some(Peer::Unix(unix_cred)) = &ctx.peer {
        println!("UID: {}", unix_cred.uid());
        println!("GID: {}", unix_cred.gid());
        
        if let Some(pid) = unix_cred.pid() {
            println!("PID: {}", pid);
            
            #[cfg(target_os = "linux")]
            if let Ok(path) = unix_cred.process_path() {
                println!("Process path: {:?}", path);
            }
        }
    }
}
```

## Example: Access Control

Here's a complete example showing how to use Context for access control:

```rust
use jetstream::prelude::*;
use jetstream_macros::service;

#[service]
pub trait SecureService {
    async fn restricted_operation(&mut self) -> Result<String, Error>;
}

struct SecureServiceImpl {
    allowed_uids: Vec<u32>,
}

impl SecureService for SecureServiceImpl {
    async fn restricted_operation(&mut self) -> Result<String, Error> {
        // Note: In a real service, you would obtain the context
        // from the framed connection when handling the request
        Ok("Operation completed".to_string())
    }
}

// In the server handler
async fn handle_connection(stream: impl std::io::Read + std::io::Write) {
    let framed = Framed::new(stream, /* codec */);
    let ctx = framed.context();
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Some(Peer::Unix(unix_cred)) = &ctx.peer {
        let allowed_uids = vec![1000, 1001];
        if !allowed_uids.contains(&unix_cred.uid()) {
            eprintln!("Access denied for UID: {}", unix_cred.uid());
            return;
        }
    }
    
    // Process the request...
}
```

## Example: Audit Logging

Using Context to log connection information:

```rust
use jetstream::prelude::*;

fn log_connection_info(ctx: &Context) {
    let mut log_msg = String::from("Connection: ");
    
    if let Some(remote) = &ctx.remote {
        match remote {
            RemoteAddr::UnixAddr(path) => {
                log_msg.push_str(&format!("Unix socket {:?}", path));
            }
            RemoteAddr::IpAddr(ip) => {
                log_msg.push_str(&format!("IP {}", ip));
            }
            RemoteAddr::NodeAddr(node_addr) => {
                log_msg.push_str(&format!("Iroh node {:?}", node_addr));
            }
        }
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Some(Peer::Unix(unix_cred)) = &ctx.peer {
        log_msg.push_str(&format!(
            " (UID: {}, GID: {}",
            unix_cred.uid(),
            unix_cred.gid()
        ));
        if let Some(pid) = unix_cred.pid() {
            log_msg.push_str(&format!(", PID: {}", pid));
        }
        log_msg.push(')');
    }
    
    println!("{}", log_msg);
}
```

## Platform Support

### Unix Domain Sockets (Linux/macOS)
- Provides full peer credentials (UID, GID, PID)
- Can retrieve process path on Linux via `/proc/{pid}/exe`
- Most detailed context information available

### QUIC Connections
- Provides remote IP address
- No peer credentials (use mTLS certificates for authentication)

### Iroh Connections
- Provides node address with optional relay URL
- Provides node ID for peer identification
- Supports direct addresses for connection info

## Best Practices

1. **Always check for None**: Context fields are `Option` types and may not always be available
2. **Platform-specific code**: Use `#[cfg]` attributes for Unix-specific functionality
3. **Security**: Don't rely solely on context for authentication; use it in combination with mTLS or other authentication mechanisms
4. **Logging**: Include context information in audit logs for debugging and security monitoring

## See Also

- [Tracing documentation](tracing.md) - For instrumenting RPC calls
- [Examples](https://github.com/sevki/jetstream/tree/main/examples) - See context in action
