//! MUD Server module
//!
//! Provides the main server functionality including:
//! - TCP listener for client connections
//! - Game loop for periodic updates
//! - Player management

pub mod game_loop;

pub use game_loop::{GameLoop, GameLoopConfig, run_game_loop};

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info};

use crate::network::broadcaster::Broadcaster;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to
    pub bind_addr: String,
    /// Port to listen on
    pub port: u16,
    /// Game loop configuration
    pub game_loop: GameLoopConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 9999,
            game_loop: GameLoopConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(bind_addr: impl Into<String>, port: u16) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            port,
            game_loop: GameLoopConfig::default(),
        }
    }

    /// Get the full bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_addr, self.port)
    }

    /// Set the game loop configuration
    pub fn with_game_loop(mut self, config: GameLoopConfig) -> Self {
        self.game_loop = config;
        self
    }
}

/// MUD Server
///
/// Manages the game server including client connections and the game loop.
pub struct MudServer {
    /// Server configuration
    config: ServerConfig,
    /// Broadcaster for broadcasting messages
    broadcaster: Arc<Broadcaster>,
}

impl MudServer {
    /// Create a new MUD server
    pub fn new(config: ServerConfig) -> Self {
        let broadcaster = Arc::new(Broadcaster::new());

        Self {
            config,
            broadcaster,
        }
    }

    /// Create a server with default configuration
    pub fn default() -> Self {
        Self::new(ServerConfig::default())
    }

    /// Get the broadcaster
    pub fn broadcaster(&self) -> Arc<Broadcaster> {
        self.broadcaster.clone()
    }

    /// Initialize the server
    ///
    /// Loads game data and registers commands
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing MUD server...");

        // Register basic commands would happen here
        // TODO: Load game data

        info!("Server initialization complete");
        Ok(())
    }

    /// Run the server
    ///
    /// This starts the TCP listener and game loop
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize().await?;

        let bind_addr = self.config.bind_address();
        let listener = TcpListener::bind(&bind_addr).await?;

        info!("MUD Server listening on {}", bind_addr);
        info!("=============================================================");
        info!("          ☞ 무크 Rust 버전 서버를 실행 합니다.");
        info!("=============================================================");

        // Spawn game loop
        let broadcaster = self.broadcaster.clone();
        let players = Arc::new(AsyncMutex::new(Vec::new()));
        let game_loop_config = self.config.game_loop.clone();
        tokio::spawn(async move {
            run_game_loop(broadcaster, players, game_loop_config).await;
        });

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);

                    let broadcaster = self.broadcaster.clone();

                    tokio::spawn(async move {
                        if let Err(e) = crate::network::client::handle_client(stream, addr, broadcaster).await {
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
}

/// Server welcome message
pub const WELCOME_MESSAGE: &str = r#"
=============================================================
          ☞ 무크 Rust 버전에 오신 것을 환영합니다!
=============================================================
"#;

/// Server shutdown message
pub const SHUTDOWN_MESSAGE: &str = "\r\n\r\n다음에 또 만나요~!!!\r\n";

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

    #[test]
    fn test_mud_server_new() {
        let server = MudServer::new(ServerConfig::default());
        // Server should be created successfully
        assert_eq!(server.config.port, 9999);
    }

    #[test]
    fn test_welcome_message() {
        assert!(WELCOME_MESSAGE.contains("무크"));
        assert!(WELCOME_MESSAGE.contains("환영"));
    }
}
