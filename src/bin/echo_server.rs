//! Echo server for testing the network module
//!
//! Simple TCP echo server that broadcasts received messages to all connected clients.

use muc_engine::network::run_echo_server;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let port = if args.len() > 1 {
        args[1].parse::<u16>().unwrap_or(9999)
    } else {
        9999
    };

    println!("========================================");
    println!("  MUC Rust Echo Server");
    println!("  Listening on 0.0.0.0:{}", port);
    println!("========================================");
    println!();
    println!("Features:");
    println!("  - Line-based protocol (delimiters: \\r\\n, \\r\\000)");
    println!("  - UTF-8 encoding with error handling");
    println!("  - Broadcast to all connected clients");
    println!();
    println!("Connect with: telnet localhost {}", port);
    println!("Type 'quit' to disconnect");
    println!("========================================");
    println!();

    // Run the echo server
    run_echo_server(port).await?;

    Ok(())
}
