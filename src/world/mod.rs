//! World module for MUD game world
//!
//! This module contains structures and functionality for managing
//! the game world including rooms, zones, mobs, items, and navigation.

pub mod room;
pub mod mob;
pub mod item;

// Re-export commonly used types
pub use room::{
    Room, RoomCache, RoomError, Direction, Exit,
    get_room, handle_player_enter, handle_player_exit,
    format_room_header, format_exits_long,
    EnterMode, ExitMode,
};

pub use mob::{
    MobCache, MobInstance, RawMobData, MobError,
    get_mob_cache,
};

pub use item::{
    ItemCache, ItemInstance, RawItemData, ItemError,
    get_item_cache, create_item, find_or_create_item,
};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::object::Object;

/// Player position in the world
#[derive(Debug, Clone)]
pub struct PlayerPosition {
    /// Zone name
    pub zone: String,
    /// Room index
    pub room: i64,
    /// Last movement timestamp
    pub last_move: i64,
}

impl PlayerPosition {
    /// Create a new position
    pub fn new(zone: String, room: i64) -> Self {
        Self {
            zone,
            room,
            last_move: chrono::Utc::now().timestamp(),
        }
    }

    /// Create starting position (낙양성:1)
    pub fn start() -> Self {
        Self::new("낙양성".to_string(), 1)
    }

    /// Get the position key for room lookup
    pub fn room_key(&self) -> String {
        format!("{}:{}", self.zone, self.room)
    }
}

/// World state containing all active entities
#[derive(Debug)]
pub struct WorldState {
    /// Player positions indexed by player name
    pub player_positions: HashMap<String, PlayerPosition>,
    /// Room cache
    pub room_cache: RoomCache,
    /// Mob cache
    pub mob_cache: MobCache,
    /// Item cache
    pub item_cache: ItemCache,
    /// Objects on room floor: key = "zone:room" (e.g. "낙양성:1"), value = items (non-stackable)
    pub room_objs: HashMap<String, Vec<Arc<Mutex<Object>>>>,
    /// Stackable items on room floor: "zone:room" -> (인덱스 -> 수량)
    pub room_inv_stack: HashMap<String, HashMap<String, i64>>,
}

impl WorldState {
    /// Create a new world state
    pub fn new() -> Self {
        Self {
            player_positions: HashMap::new(),
            room_cache: RoomCache::new(),
            mob_cache: MobCache::new(),
            item_cache: ItemCache::new(),
            room_objs: HashMap::new(),
            room_inv_stack: HashMap::new(),
        }
    }

    /// Get or create the list of objects on a room's floor. Key: "zone:room".
    pub fn get_room_objs_mut(&mut self, zone: &str, room: i64) -> &mut Vec<Arc<Mutex<Object>>> {
        let key = format!("{}:{}", zone, room);
        self.room_objs.entry(key).or_default()
    }

    /// Get a copy of the list of objects on a room's floor (for display). Key: "zone:room".
    pub fn get_room_objs(&self, zone: &str, room: i64) -> Vec<Arc<Mutex<Object>>> {
        let key = format!("{}:{}", zone, room);
        self.room_objs.get(&key).cloned().unwrap_or_default()
    }

    /// Get or create stackable items on a room's floor. Key: "zone:room", inner: 인덱스 -> 수량.
    pub fn get_room_objs_stack_mut(&mut self, zone: &str, room: i64) -> &mut HashMap<String, i64> {
        let key = format!("{}:{}", zone, room);
        self.room_inv_stack.entry(key).or_default()
    }

    /// Get stackable counts on a room's floor (for display).
    pub fn get_room_objs_stack(&self, zone: &str, room: i64) -> HashMap<String, i64> {
        let key = format!("{}:{}", zone, room);
        self.room_inv_stack.get(&key).cloned().unwrap_or_default()
    }

    /// Initialize the world (load initial data)
    ///
    /// 방/몹은 사용자 진입 시점에 get_room, spawn_mobs_for_room 경로로 동적 로딩.
    /// 서버 기동 시 낙양성 전체를 프리로드하지 않아 메모리 절약.
    pub fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Get a player's position
    pub fn get_player_position(&self, player_name: &str) -> Option<&PlayerPosition> {
        self.player_positions.get(player_name)
    }

    /// Set a player's position
    pub fn set_player_position(&mut self, player_name: &str, pos: PlayerPosition) {
        self.player_positions.insert(player_name.to_string(), pos);
    }

    /// Remove a player's position (e.g. when kicked due to duplicate login)
    pub fn remove_player_position(&mut self, player_name: &str) -> Option<PlayerPosition> {
        self.player_positions.remove(player_name)
    }

    /// Move a player in a direction
    pub fn move_player(
        &mut self,
        player_name: &str,
        direction: Direction,
    ) -> Result<(String, i64), String> {
        let current_pos = self.player_positions.get(player_name)
            .ok_or("Player not in world")?;

        // Get current room
        let room = self.room_cache.get_room(&current_pos.zone, &current_pos.room.to_string())
            .map_err(|e| format!("Failed to get room: {}", e))?;

        let room_read = room.read().unwrap();
        let exit = room_read.get_exit(direction)
            .ok_or(format!("{}쪽으로 갈 수 없습니다.", direction.korean_name()))?;

        let dest = exit.destination(&current_pos.zone)
            .ok_or("Invalid exit destination")?;

        // Update player position
        let new_pos = PlayerPosition::new(dest.0.clone(), dest.1);
        self.player_positions.insert(player_name.to_string(), new_pos.clone());

        Ok((dest.0, dest.1))
    }

    /// 고유 명칭 또는 방향명("초보수련장", "출구", "북" 등)으로 이동.
    /// 반환: (new_zone, new_room, 이동 메시지용 이름 "북쪽" or "초보수련장")
    pub fn move_player_by_name(
        &mut self,
        player_name: &str,
        exit_name: &str,
    ) -> Result<(String, i64, String), String> {
        let current_pos = self.player_positions.get(player_name)
            .ok_or("Player not in world")?;

        let room = self.room_cache.get_room(&current_pos.zone, &current_pos.room.to_string())
            .map_err(|e| format!("Failed to get room: {}", e))?;

        let (dest, msg_name) = {
            let room_read = room.read().unwrap();
            let exit = room_read.get_exit_by_name(exit_name)
                .ok_or_else(|| format!("{} (으)로 갈 수 없습니다.", exit_name))?;
            let d = exit.destination("").ok_or("Invalid exit destination")?;
            (d, exit.exit_message_name().to_string())
        };

        let new_pos = PlayerPosition::new(dest.0.clone(), dest.1);
        self.player_positions.insert(player_name.to_string(), new_pos);
        self.spawn_mobs_for_room(&dest.0, dest.1);
        Ok((dest.0, dest.1, msg_name))
    }

    /// Get mobs in a player's current room
    pub fn get_mobs_for_player(&self, player_name: &str) -> Vec<&MobInstance> {
        if let Some(pos) = self.player_positions.get(player_name) {
            self.mob_cache.get_mobs_in_room(&pos.zone, pos.room)
        } else {
            Vec::new()
        }
    }

    /// 같은 방에 있는 플레이어 이름 목록. 파이썬 room.objs 중 is_player, 봐/이동 시 다른 유저 표시용.
    pub fn get_players_in_room(&self, zone: &str, room: i64) -> Vec<String> {
        self.player_positions
            .iter()
            .filter(|(_, pos)| pos.zone == zone && pos.room == room)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Spawn mobs for a room from 맵정보.몹 (called when player enters, on-demand load).
    pub fn spawn_mobs_for_room(&mut self, zone: &str, room: i64) {
        let mob_ids = self
            .room_cache
            .get_room(zone, &room.to_string())
            .ok()
            .and_then(|r| r.read().ok().map(|g| g.mob_ids.clone()))
            .unwrap_or_default();
        self.mob_cache.spawn_mobs_for_room(zone, room, &mob_ids);
    }

    /// Update world state (respawns, etc.)
    pub fn update(&mut self) {
        self.mob_cache.update_respawns();
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global world state accessor
pub fn get_world_state() -> &'static RwLock<WorldState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<RwLock<WorldState>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut world = WorldState::new();
        // Try to initialize, log error but don't panic
        if let Err(e) = world.initialize() {
            eprintln!("Failed to initialize world: {}", e);
        }
        RwLock::new(world)
    })
}
