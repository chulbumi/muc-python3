//! MUD Server module
//!
//! Provides the main server functionality including:
//! - TCP listener for client connections
//! - Game loop for periodic updates
//! - Player management

pub mod game_loop;

pub use game_loop::{GameLoop, GameLoopConfig, run_game_loop};

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::Notify;
use tracing::{error, info};

use crate::command::commands::register_basic_commands;
use crate::command::commands::script::register_script_commands;
use crate::command::CommandRegistry;
use crate::hotreload::{HotReloadManager, ReloadConfig};
use crate::network::broadcaster::Broadcaster;
use crate::network::client::{get_other_players_desc_in_room, get_other_players_map_for_look};
use crate::scheduler::CallOutScheduler;
use crate::script::{create_call_out_script_runner, ScriptConfig, ScriptStorage};
use crate::world::{get_world_state, RoomCache};

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

        // command_registry, script_storage, room_cache, call_out_scheduler를 한 번만 생성
        let get_other_players_desc: Arc<dyn Fn(&str) -> Vec<String> + Send + Sync> = Arc::new({
            let bc = self.broadcaster.clone();
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
        let script_runner = create_call_out_script_runner(script_storage.clone(), self.broadcaster.clone());
        let call_out_scheduler = Arc::new(CallOutScheduler::new(
            self.broadcaster.clone(),
            Duration::from_millis(100),
            Some(script_runner),
        ));

        // cmds/*.rhai Hot Reload: 1초마다 변경된 스크립트만 다시 로드
        HotReloadManager::new(script_storage.clone(), ReloadConfig::default()).start_watcher();

        let mut registry = CommandRegistry::new();
        register_basic_commands(&mut registry);
        register_script_commands(
            &mut registry,
            script_storage,
            Some(get_other_players_desc),
            Some(get_other_players_map),
            Some(call_out_scheduler.clone()),
        )
        .await;
        let command_registry = Arc::new(registry);
        let room_cache = Arc::new(Mutex::new(RoomCache::new()));

        // Spawn game loop (process_due에서 call_out 만료 시 스크립트 함수 실행)
        let broadcaster = self.broadcaster.clone();
        let players = Arc::new(AsyncMutex::new(Vec::new()));
        let game_loop_config = self.config.game_loop.clone();
        let call_out_for_loop = call_out_scheduler.clone();
        tokio::spawn(async move {
            run_game_loop(broadcaster, players, game_loop_config, Some(call_out_for_loop)).await;
        });

        // 셧다운/CTRL+C/kill(SIGTERM) 시 서버 종료 시퀀스 트리거
        let shutdown_notify = Arc::new(Notify::new());
        let sn = shutdown_notify.clone();
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {}
                        _ = sigterm.recv() => {}
                    }
                } else {
                    let _ = tokio::signal::ctrl_c().await;
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            sn.notify_waiters();
        });

        // Accept connections (shutdown_notify 시 루프 탈출)
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!("New connection from {}", addr);
                            let broadcaster = self.broadcaster.clone();
                            let command_registry = command_registry.clone();
                            let room_cache = room_cache.clone();
                            let shutdown_notify = shutdown_notify.clone();
                            tokio::spawn(async move {
                                if let Err(e) = crate::network::client::handle_client(
                                    stream, addr, broadcaster, command_registry, room_cache,
                                    Some(shutdown_notify),
                                ).await {
                                    error!("Error handling client {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}", e);
                        }
                    }
                }
                _ = shutdown_notify.notified() => {
                    info!("Shutdown requested (CTRL+C / kill / 셧다운)");
                    break;
                }
            }
        }

        // 셧다운 시퀀스: 전체 사용자에게 종료 안내 → 연결 해제 요청 → 잠시 대기 후 종료
        let msg = "\r\n\r\n\x1b[1;33m☞ 서버가 종료됩니다. 다음에 또 만나요~!!!\x1b[0;37m\r\n";
        self.broadcaster.broadcast(msg, None);
        for addr in self.broadcaster.client_addresses() {
            let _ = self.broadcaster.request_disconnect(addr);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        info!("Server shutdown complete");
        Ok(())
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
