//! Game loop for MUD server
//!
//! Handles periodic updates:
//! - Player idle timeout checks
//! - Player updates
//! - Room updates
//! - Mob movement updates
//! - Zone updates (every 60 ticks)

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::interval;
use tracing::debug;

use crate::network::Broadcaster;
use crate::player::Player;
use crate::scheduler::CallOutScheduler;

/// Game loop configuration
#[derive(Debug, Clone)]
pub struct GameLoopConfig {
    /// Tick interval in seconds (default: 1 second)
    pub tick_interval: Duration,
    /// Idle timeout for INACTIVE players (default: 10 seconds)
    pub inactive_timeout: u64,
    /// Idle timeout for ACTIVE players (default: 180 seconds)
    pub active_timeout: u64,
    /// Zone update interval in ticks (default: 60 ticks)
    pub zone_update_interval: u32,
}

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
            inactive_timeout: 10,
            active_timeout: 180,
            zone_update_interval: 60,
        }
    }
}

/// Game loop state
pub struct GameLoop {
    /// Configuration
    config: GameLoopConfig,
    /// Current tick counter
    tick_count: u32,
    /// List of zones to update
    zone_list: Vec<String>,
    /// Current zone index
    zone_index: usize,
    /// Last tick time
    last_tick: Instant,
}

impl GameLoop {
    /// Create a new game loop
    pub fn new(config: GameLoopConfig) -> Self {
        Self {
            config,
            tick_count: 0,
            zone_list: Vec::new(),
            zone_index: 0,
            last_tick: Instant::now(),
        }
    }

    /// Create a game loop with default configuration
    pub fn default() -> Self {
        Self::new(GameLoopConfig::default())
    }

    /// Run a single tick
    ///
    /// Returns true if the loop should continue
    pub fn tick(&mut self, players: &mut Vec<Arc<AsyncMutex<Player>>>) -> bool {
        self.tick_count += 1;

        // Update zones every 60 ticks
        if self.tick_count % self.config.zone_update_interval == 0 {
            self.update_zones();
        }

        let mut players_to_disconnect = Vec::new();

        // Collect player data for this tick
        for player_arc in players.iter() {
            let mut player = match player_arc.try_lock() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Check idle timeouts (using idle counter)
            let should_disconnect = match player.state {
                crate::player::STATE_INACTIVE if player.idle >= self.config.inactive_timeout => {
                    true
                }
                _ if player.state != crate::player::STATE_INACTIVE
                    && player.idle >= self.config.active_timeout =>
                {
                    true
                }
                _ => false,
            };

            if should_disconnect {
                // Store player name for disconnect message
                let name = player.body.get_name();
                players_to_disconnect.push(name);
                continue;
            }

            // Increment idle counter
            player.idle += 1;
        }

        // Note: In a real implementation, disconnect logic would be handled
        // by the connection handler, not the game loop

        // Update rooms (simplified)
        if self.tick_count % 5 == 0 {
            self.update_rooms();
        }

        // Update moving mobs
        self.update_movings();

        // Calculate sleep time to maintain 1-second tick
        let elapsed = self.last_tick.elapsed();
        self.last_tick = Instant::now();

        if elapsed >= Duration::from_secs(1) {
            debug!(
                "Tick {} took longer than 1 second: {:?}",
                self.tick_count, elapsed
            );
        }

        true
    }

    /// Update one zone (called every 60 ticks)
    fn update_zones(&mut self) {
        // This would update zone data
        // For now, just log
        debug!("Zone update tick {}", self.tick_count);
    }

    /// Update rooms (simplified)
    fn update_rooms(&self) {
        // Room update logic would go here
        debug!("Updating rooms");
    }

    /// Update moving mobs
    fn update_movings(&self) {
        // This would update mob movement
        // For now, just a placeholder
        debug!("Updating moving mobs");
    }
}

/// Run the game loop asynchronously.
/// call_out_scheduler: Some이면 매 틱 process_due() 호출 (지연 스크립트 함수 실행).
pub async fn run_game_loop(
    _broadcaster: Arc<Broadcaster>,
    players: Arc<AsyncMutex<Vec<Arc<AsyncMutex<Player>>>>>,
    config: GameLoopConfig,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
) {
    let mut timer = interval(config.tick_interval);
    timer.tick().await; // Skip first immediate tick

    let mut game_loop = GameLoop::new(config);

    loop {
        timer.tick().await;

        if let Some(s) = &call_out_scheduler {
            let _ = s.process_due();
        }

        let mut players_guard = players.lock().await;
        game_loop.tick(&mut *players_guard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_loop_config_default() {
        let config = GameLoopConfig::default();
        assert_eq!(config.tick_interval, Duration::from_secs(1));
        assert_eq!(config.inactive_timeout, 10);
        assert_eq!(config.active_timeout, 180);
        assert_eq!(config.zone_update_interval, 60);
    }

    #[test]
    fn test_game_loop_new() {
        let game_loop = GameLoop::new(GameLoopConfig::default());
        assert_eq!(game_loop.tick_count, 0);
        assert!(game_loop.zone_list.is_empty());
    }

    #[test]
    fn test_game_loop_tick() {
        let mut game_loop = GameLoop::new(GameLoopConfig::default());
        let mut players = Vec::new();

        // First tick
        game_loop.tick(&mut players);
        assert_eq!(game_loop.tick_count, 1);

        // 60 ticks should trigger zone update
        for _ in 1..60 {
            game_loop.tick(&mut players);
        }
        assert_eq!(game_loop.tick_count, 60);
    }

    #[test]
    fn test_zone_update_interval() {
        let config = GameLoopConfig {
            zone_update_interval: 10,
            ..Default::default()
        };
        let mut game_loop = GameLoop::new(config);
        let mut players = Vec::new();

        // Zone update should happen at tick 10, 20, 30, etc.
        for i in 1..=30 {
            game_loop.tick(&mut players);
            if i % 10 == 0 {
                // Zone update tick
            }
        }
        assert_eq!(game_loop.tick_count, 30);
    }
}
