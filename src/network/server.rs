//! TCP server for MUD connections
//!
//! Provides an asynchronous TCP server using tokio for handling multiple client connections.

use std::sync::Arc;
use std::sync::Mutex;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber;

use crate::command::commands::register_basic_commands;
use crate::command::commands::script::register_script_commands;
use crate::command::CommandRegistry;
use crate::network::broadcaster::Broadcaster;
use crate::network::client::{get_other_players_desc_in_room, get_other_players_map_for_look, handle_client};
use crate::script::{ScriptConfig, ScriptStorage};
use crate::world::{get_world_state, RoomCache};

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

    let get_other_players_desc: Arc<dyn Fn(&str) -> Vec<String> + Send + Sync> = Arc::new({
        let bc = broadcaster.clone();
        move |exclude: &str| {
            let world = get_world_state().read().unwrap();
            let pos = match world.get_player_position(exclude) {
                Some(p) => p.clone(),
                None => return vec![],
            };
            get_other_players_desc_in_room(bc.as_ref(), &pos.zone, &pos.room, exclude)
        }
    });
    let get_other_players_map: Arc<dyn Fn() -> std::collections::HashMap<String, String> + Send + Sync> =
        Arc::new(get_other_players_map_for_look);
    let script_storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::new(ScriptConfig::default())));
    let mut registry = CommandRegistry::new();
    register_basic_commands(&mut registry);
    register_script_commands(&mut registry, script_storage, Some(get_other_players_desc), Some(get_other_players_map), None).await;
    let command_registry = Arc::new(registry);
    let room_cache = Arc::new(Mutex::new(RoomCache::new()));

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();
                let command_registry = command_registry.clone();
                let room_cache = room_cache.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone, command_registry, room_cache, None).await {
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

    let get_other_players_desc: Arc<dyn Fn(&str) -> Vec<String> + Send + Sync> = Arc::new({
        let bc = broadcaster.clone();
        move |exclude: &str| {
            let world = get_world_state().read().unwrap();
            let pos = match world.get_player_position(exclude) {
                Some(p) => p.clone(),
                None => return vec![],
            };
            get_other_players_desc_in_room(bc.as_ref(), &pos.zone, &pos.room, exclude)
        }
    });
    let get_other_players_map: Arc<dyn Fn() -> std::collections::HashMap<String, String> + Send + Sync> =
        Arc::new(get_other_players_map_for_look);
    let script_storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::new(ScriptConfig::default())));
    let mut registry = CommandRegistry::new();
    register_basic_commands(&mut registry);
    register_script_commands(&mut registry, script_storage, Some(get_other_players_desc), Some(get_other_players_map), None).await;
    let command_registry = Arc::new(registry);
    let room_cache = Arc::new(Mutex::new(RoomCache::new()));

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();
                let command_registry = command_registry.clone();
                let room_cache = room_cache.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone, command_registry, room_cache, None).await {
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

    let get_other_players_desc: Arc<dyn Fn(&str) -> Vec<String> + Send + Sync> = Arc::new({
        let bc = broadcaster.clone();
        move |exclude: &str| {
            let world = get_world_state().read().unwrap();
            let pos = match world.get_player_position(exclude) {
                Some(p) => p.clone(),
                None => return vec![],
            };
            get_other_players_desc_in_room(bc.as_ref(), &pos.zone, &pos.room, exclude)
        }
    });
    let get_other_players_map: Arc<dyn Fn() -> std::collections::HashMap<String, String> + Send + Sync> =
        Arc::new(get_other_players_map_for_look);
    let script_storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::new(ScriptConfig::default())));
    let mut registry = CommandRegistry::new();
    register_basic_commands(&mut registry);
    register_script_commands(&mut registry, script_storage, Some(get_other_players_desc), Some(get_other_players_map), None).await;
    let command_registry = Arc::new(registry);
    let room_cache = Arc::new(Mutex::new(RoomCache::new()));

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("New connection from {}", addr);

                let broadcaster_clone = broadcaster.clone();
                let command_registry = command_registry.clone();
                let room_cache = room_cache.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, broadcaster_clone, command_registry, room_cache, None).await {
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
