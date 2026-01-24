//! TCP server for MUD connections
//!
//! Provides an asynchronous TCP server using tokio for handling multiple client connections.

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber;

use crate::network::{broadcaster::Broadcaster, client::handle_client};

/// TCP Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to
    pub bind_addr: String,
    /// Port to listen on
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 9999,
        }
    }
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(bind_addr: impl Into<String>, port: u16) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            port,
        }
    }

    /// Get the full bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_addr, self.port)
    }
}

/// Run the TCP server
///
/// This function binds to the specified address and port,
/// then accepts incoming connections and spawns tasks to handle each client.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let bind_addr = config.bind_address();
    let listener = TcpListener::bind(&bind_addr).await?;

    info!("MUD Server listening on {}", bind_addr);

    let broadcaster = Arc::new(Broadcaster::new());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone).await {
                        error!("Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Run the TCP server with a custom broadcaster
///
/// This variant allows passing a pre-configured broadcaster instance.
pub async fn run_server_with_broadcaster(
    config: ServerConfig,
    broadcaster: Arc<Broadcaster>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = config.bind_address();
    let listener = TcpListener::bind(&bind_addr).await?;

    info!("MUD Server listening on {}", bind_addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone).await {
                        error!("Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Run a simple echo server for testing
///
/// This server echoes back any messages it receives to all connected clients.
pub async fn run_echo_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind_addr).await?;

    println!("Echo Server listening on {}", bind_addr);

    let broadcaster = Arc::new(Broadcaster::new());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone).await {
                        eprintln!("Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0");
        assert_eq!(config.port, 9999);
    }

    #[test]
    fn test_server_config_new() {
        let config = ServerConfig::new("127.0.0.1", 8080);
        assert_eq!(config.bind_addr, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_bind_address() {
        let config = ServerConfig::new("127.0.0.1", 8080);
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }
}
