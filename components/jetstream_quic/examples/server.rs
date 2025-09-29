use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use the localhost certificate we generated for Chrome
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/localhost.pem");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/localhost.key");

    // Load server certificate
    let cert_chain = fs::read(Path::new(cert_path))?;
    let cert = rustls_pemfile::certs(&mut &*cert_chain)
        .collect::<Result<Vec<_>, _>>()?
        .pop()
        .ok_or("No certificate found")?;

    // Load server private key
    let key_data = fs::read(Path::new(key_path))?;
    let key = rustls_pemfile::private_key(&mut &*key_data)?
        .ok_or("No private key found")?;

    // Create and run the server on port 4433 (as expected by launch_chrome.sh)
    let server = jetstream_quic::Server::new_with_addr(cert, key, "127.0.0.1:4433");
    println!("QUIC server listening on 127.0.0.1:4433");
    println!("You can now run ./launch_chrome.sh to connect with Chrome");
    server.run().await;

    Ok(())
}